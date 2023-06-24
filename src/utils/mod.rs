pub(crate) mod azure_api;
pub mod log;

use rand::Rng;

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
    ret_str
}

/// 二进制数组查询
pub fn binary_search(bin: &[u8], search: &[u8]) -> Option<usize> {
    if bin.len() > usize::MAX - search.len() {
        panic!("binary_search: length overflow");
    }
    let mut i = 0;
    let k: usize = bin.len() - search.len();
    loop {
        if i > k {
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
