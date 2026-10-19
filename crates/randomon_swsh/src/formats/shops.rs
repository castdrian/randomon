use crate::bytes::put_i32;
use crate::error::{Result, invalid};
use crate::formats::flatbuffer::{TableBuilder, TableRef, VectorRef};

pub const FILE: &str = "bin/appli/shop/bin/shop_data.bin";
const CONTEXT: &str = "ShopInventory";
const ITEM_SIZE: usize = 4;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SingleShop {
    pub hash: u64,
    pub items: Vec<i32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MultiShop {
    pub hash: u64,
    pub inventories: Vec<Vec<i32>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopInventory {
    pub single: Vec<SingleShop>,
    pub multi: Vec<MultiShop>,
    original_hashes: Vec<u64>,
    item_vectors: Vec<(usize, usize)>,
    raw: Vec<u8>,
}

fn inventory_items(inventory: Option<TableRef<'_>>) -> Result<(Vec<i32>, Option<VectorRef<'_>>)> {
    let Some(inventory) = inventory else {
        return Ok((Vec::new(), None));
    };
    inventory.ensure_known_fields(1, CONTEXT)?;
    let vector = inventory.vector(0)?;
    let items = match vector {
        Some(vector) => vector.scalars()?,
        None => Vec::new(),
    };
    Ok((items, vector))
}

fn record_vector(vectors: &mut Vec<(usize, usize)>, vector: Option<VectorRef<'_>>) {
    if let Some(vector) = vector {
        vectors.push((vector.start(), vector.len()));
    }
}

impl ShopInventory {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let root = TableRef::root(data)?;
        root.ensure_known_fields(2, CONTEXT)?;
        let mut item_vectors = Vec::new();
        let mut single = Vec::new();
        for shop in root.tables(0)? {
            shop.ensure_known_fields(2, CONTEXT)?;
            let (items, vector) = inventory_items(shop.table(1)?)?;
            record_vector(&mut item_vectors, vector);
            single.push(SingleShop {
                hash: shop.scalar(0)?,
                items,
            });
        }
        let mut multi = Vec::new();
        for shop in root.tables(1)? {
            shop.ensure_known_fields(2, CONTEXT)?;
            let mut inventories = Vec::new();
            for inventory in shop.tables(1)? {
                let (items, vector) = inventory_items(Some(inventory))?;
                record_vector(&mut item_vectors, vector);
                inventories.push(items);
            }
            multi.push(MultiShop {
                hash: shop.scalar(0)?,
                inventories,
            });
        }
        let original_hashes = single
            .iter()
            .map(|shop| shop.hash)
            .chain(multi.iter().map(|shop| shop.hash))
            .collect();
        Ok(ShopInventory {
            single,
            multi,
            original_hashes,
            item_vectors,
            raw: data.to_vec(),
        })
    }

    fn hashes(&self) -> Vec<u64> {
        self.single
            .iter()
            .map(|shop| shop.hash)
            .chain(self.multi.iter().map(|shop| shop.hash))
            .collect()
    }

    fn item_lists(&self) -> Vec<&Vec<i32>> {
        self.single
            .iter()
            .map(|shop| &shop.items)
            .chain(self.multi.iter().flat_map(|shop| shop.inventories.iter()))
            .collect()
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        if self.hashes() != self.original_hashes {
            return Err(invalid(
                CONTEXT,
                "shops cannot be added, removed or renamed in place",
            ));
        }
        let lists: Vec<&Vec<i32>> = self
            .item_lists()
            .into_iter()
            .filter(|list| !list.is_empty())
            .collect();
        let vectors: Vec<&(usize, usize)> = self
            .item_vectors
            .iter()
            .filter(|(_, length)| *length > 0)
            .collect();
        if lists.len() != vectors.len()
            || lists
                .iter()
                .zip(&vectors)
                .any(|(list, (_, length))| list.len() != *length)
        {
            return Err(invalid(CONTEXT, "inventory lengths must stay the same"));
        }
        let mut data = self.raw.clone();
        for (list, (start, _)) in lists.iter().zip(vectors) {
            for (index, item) in list.iter().enumerate() {
                put_i32(&mut data, start + index * ITEM_SIZE, *item, CONTEXT)?;
            }
        }
        Ok(data)
    }

    pub fn build(single: &[SingleShop], multi: &[MultiShop]) -> Vec<u8> {
        let inventory = |items: &[i32]| {
            let mut builder = TableBuilder::new();
            builder.scalars(0, items);
            builder
        };
        let single_tables = single
            .iter()
            .map(|shop| {
                let mut builder = TableBuilder::new();
                builder
                    .scalar(0, shop.hash)
                    .table(1, inventory(&shop.items));
                builder
            })
            .collect();
        let multi_tables = multi
            .iter()
            .map(|shop| {
                let mut builder = TableBuilder::new();
                builder.scalar(0, shop.hash).tables(
                    1,
                    shop.inventories
                        .iter()
                        .map(|items| inventory(items))
                        .collect(),
                );
                builder
            })
            .collect();
        let mut root = TableBuilder::new();
        root.tables(0, single_tables).tables(1, multi_tables);
        root.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        ShopInventory::build(
            &[
                SingleShop {
                    hash: 0x1F3F_F031_A3A2_4490,
                    items: vec![17, 18, 19],
                },
                SingleShop {
                    hash: 2,
                    items: Vec::new(),
                },
            ],
            &[MultiShop {
                hash: 3,
                inventories: vec![vec![4], vec![4, 3, 17]],
            }],
        )
    }

    #[test]
    fn unmodified_shops_round_trip_exactly() {
        let data = sample();
        let shops = ShopInventory::parse(&data).expect("parse");
        assert_eq!(shops.single[0].items, vec![17, 18, 19]);
        assert_eq!(shops.multi[0].inventories[1], vec![4, 3, 17]);
        assert_eq!(shops.to_bytes().expect("write"), data);
    }

    #[test]
    fn items_are_replaced_in_place() {
        let data = sample();
        let mut shops = ShopInventory::parse(&data).expect("parse");
        shops.single[0].items[1] = 50;
        shops.multi[0].inventories[1][2] = 28;
        let written = shops.to_bytes().expect("write");
        assert_eq!(written.len(), data.len());
        let reparsed = ShopInventory::parse(&written).expect("parse");
        assert_eq!(reparsed.single[0].items, vec![17, 50, 19]);
        assert_eq!(reparsed.multi[0].inventories[1], vec![4, 3, 28]);
    }

    #[test]
    fn structural_changes_are_rejected() {
        let mut shops = ShopInventory::parse(&sample()).expect("parse");
        shops.single[0].items.push(1);
        assert!(shops.to_bytes().is_err());
        let mut shops = ShopInventory::parse(&sample()).expect("parse");
        shops.multi[0].hash = 9;
        assert!(shops.to_bytes().is_err());
    }
}
