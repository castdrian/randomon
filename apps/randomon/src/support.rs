pub const GITHUB_SPONSORS_URL: &str = "https://github.com/sponsors/castdrian";
pub const KO_FI_URL: &str = "https://ko-fi.com/castdrian";
pub const DISCORD_URL: &str = "https://discord.gg/NdaBaxFKnn";
pub const REPOSITORY_URL: &str = "https://github.com/castdrian/randomon";

pub fn open_link(url: &str) {
    if let Err(error) = open::that_detached(url) {
        log::warn!("failed to open {url}: {error}");
    }
}
