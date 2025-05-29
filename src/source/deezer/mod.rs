pub mod model;
pub mod source;
pub mod stream;

static _SHARE_URL: &str = "https://deezer.page.link/";
static PUBLIC_API_BASE: &str = "https://api.deezer.com/2.0";
static PRIVATE_API_BASE: &str = "https://www.deezer.com/ajax/gw-light.php";
static MEDIA_BASE: &str = "https://media.deezer.com/v1";
static SECRET_IV: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
