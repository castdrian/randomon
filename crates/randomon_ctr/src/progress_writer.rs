use std::io::{self, Read, Seek, SeekFrom, Write};

const REPORT_STEP: f32 = 0.005;
const MEGABYTE: u64 = 1024 * 1024;

pub struct ProgressWriter<'a, W> {
    inner: W,
    position: u64,
    furthest: u64,
    expected: u64,
    last_reported: f32,
    on_progress: &'a dyn Fn(f32, &str),
}

impl<'a, W> ProgressWriter<'a, W> {
    pub fn new(inner: W, expected: u64, on_progress: &'a dyn Fn(f32, &str)) -> Self {
        Self {
            inner,
            position: 0,
            furthest: 0,
            expected: expected.max(1),
            last_reported: 0.0,
            on_progress,
        }
    }

    pub fn into_inner(self) -> W {
        self.inner
    }

    fn describe(&self) -> String {
        format!(
            "Writing game image · {} of {} MB",
            self.furthest / MEGABYTE,
            self.expected / MEGABYTE
        )
    }

    fn advance(&mut self, position: u64) {
        self.position = position;
        if position <= self.furthest {
            return;
        }
        self.furthest = position;
        let fraction = (self.furthest as f32 / self.expected as f32).min(0.99);
        if fraction - self.last_reported >= REPORT_STEP {
            self.last_reported = fraction;
            (self.on_progress)(fraction, &self.describe());
        }
    }
}

impl<W: Write> Write for ProgressWriter<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buffer)?;
        self.advance(self.position + written as u64);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W: Read> Read for ProgressWriter<'_, W> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buffer)?;
        self.position += read as u64;
        Ok(read)
    }
}

impl<W: Seek> Seek for ProgressWriter<'_, W> {
    fn seek(&mut self, target: SeekFrom) -> io::Result<u64> {
        let position = self.inner.seek(target)?;
        self.position = position;
        Ok(position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::io::Cursor;

    #[test]
    fn reports_the_furthest_written_offset_without_going_backwards() {
        let seen = RefCell::new(Vec::new());
        let record = |fraction: f32, _: &str| seen.borrow_mut().push(fraction);
        let mut writer = ProgressWriter::new(Cursor::new(Vec::new()), 100, &record);
        writer.write_all(&[0; 50]).expect("write");
        writer.seek(SeekFrom::Start(0)).expect("seek");
        writer.write_all(&[1; 10]).expect("rewrite header");
        writer.seek(SeekFrom::Start(50)).expect("seek");
        writer.write_all(&[0; 60]).expect("write rest");
        assert_eq!(writer.into_inner().into_inner().len(), 110);
        let seen = seen.into_inner();
        assert_eq!(seen.len(), 2);
        assert!((seen[0] - 0.5).abs() < f32::EPSILON);
        assert!((seen[1] - 0.99).abs() < f32::EPSILON);
    }
}
