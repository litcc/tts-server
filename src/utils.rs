use rand::Rng;
use rustls_native_certs::load_native_certs;
use tokio_rustls::rustls::{Certificate, ClientConfig, RootCertStore};

/// 生成随机字符
///
/// Examples
///
/// ```
/// let x = random_string(32);
/// ```
pub fn random_string(num: u32) -> String {
    // println!("num: {} ", num);
    let str = "123456789abcdef".chars().collect::<Vec<char>>();
    let mut ret_str = String::new();
    for _i in 0..num {
        let nums = rand::thread_rng().gen_range(0..str.len());
        let k = str[nums];
        ret_str.push(k);
        // println!("添加: {} , 字符串总共: {}", k, ret_str);
    }
    return ret_str;
}

/// 获取系统跟证书
///
pub fn get_system_ca_config() -> ClientConfig {
    let mut root_store = RootCertStore::empty();
    for cert in load_native_certs().expect("could not load platform certs") {
        root_store.add(&Certificate(cert.0)).unwrap();
    }
    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    return config;
}

// 二进制数组查询
pub fn binary_search(bin: &[u8], search: &[u8]) -> Option<usize> {
    if bin.len() > usize::MAX - search.len() {
        panic!("binary_search: length overflow");
    }
    let mut i = 0;
    let k: usize = bin.len() - search.len();
    loop {
        if i >= k {
            break;
        }
        let j = i + search.len();
        if &bin[i..j] == search {
            return Some(i);
        }
        i += 1;
    }
    None
}
