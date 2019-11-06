
use iota_signing::*;
use iota_conversion::Trinary;
use iota_constants;

use iota_crypto::{Kerl, Sponge};
use iota_model::Bundle;
use iota_constants::HASH_TRINARY_SIZE;
use std::cmp;


fn sum(a: i8, b: i8) -> i8 {
    let s = a + b;
    match s {
        2 => -1,
        -2 => 1,
        _ => s,
    }
}

fn cons(a: i8, b: i8) -> i8 {
    if a == b {
        a
    } else {
        0
    }
}

fn any(a: i8, b: i8) -> i8 {
    let s = a + b;
    if s > 0 {
        1
    } else if s < 0 {
        -1
    } else {
        0
    }
}

fn full_add(a: i8, b: i8, c: i8) -> (i8, i8) {
    let s_a = sum(a, b);
    let c_a = cons(a, b);
    let c_b = cons(s_a, c);
    let c_out = any(c_a, c_b);
    let s_out = sum(s_a, c);

    (s_out, c_out)
}

/// Adds two slices of trits and returns the result
pub fn add(a: &[i8], b: &[i8]) -> Vec<i8> {
    let mut out = vec![0; cmp::max(a.len(), b.len())];
    let mut carry = 0;
    let mut a_i: i8;
    let mut b_i: i8;

    for i in 0..out.len() {
        a_i = if i < a.len() { a[i] } else { 0 };
        b_i = if i < b.len() { b[i] } else { 0 };
        let f_a = full_add(a_i, b_i, carry);
        out[i] = f_a.0;
        carry = f_a.1;
    }

    out
}



const TEST_SEED: &str =
        "IHDEENZYITYVYSPKAURUZAQKGVJEREFDJMYTANNXXGPZ9GJWTEOJJ9IPMXOGZNQLSNMFDSQOTZAEETUEA";

const TEST_TRUNK: &str =
        "BBBEENZYITYVYSPKAURUZAQKGVJEREFDJMYTANAAAGPZ9GJWTEOJJ9IPMXOGZNQLSNMFDSQOTZAEETUEA";


pub fn sign_tw_hash(seed:&str, index:usize, tw_hash:&str) -> String {
    let key_a = key(&seed.trits(), index, 1).unwrap();
    let normalized_hash = Bundle::normalized_bundle(&tw_hash);
    let signature = signature_fragment(&normalized_hash[0..27], &key_a[0..6561]).unwrap();
    signature.trytes().unwrap()
}

pub fn validate_tw_signature(pub_key:&str, tw_hash:&str, signature:&str) -> bool {
    validate_signatures(pub_key, &[String::from(signature)], tw_hash).unwrap()
}


pub fn timewarp_hash(address: &str, trunk_or_branch:&str, tag:&str) -> String {
    let mut a = address.trits();
    a.append(&mut trunk_or_branch.trits());
    let padded_tag = format!("{}{}",tag, "999999999999999999999999999999999999999999999999999999");
    a.append(&mut padded_tag.trits());
    let mut curl = Kerl::default();
    let mut hash_trits = [0; HASH_TRINARY_SIZE];       
    let _l = curl.absorb(&a);        
    let _l = curl.squeeze(&mut hash_trits);
    let hash_trytes = hash_trits.trytes().unwrap();
    hash_trytes
}

pub fn calculate_normalized_timewarp_hash(address: &str, trunk_or_branch:&str) -> (String, String) {    
    let mut valid_to_sign = false;
    let mut to_return = String::from("");
    let mut tag = "999999999999999999999999999999999999999999999999999999999999999999999999999999999".trits();
    while !valid_to_sign {        
       let hash_trytes = timewarp_hash(address, trunk_or_branch, &tag.trytes().unwrap()[0..27]);
       let normalized = Bundle::normalized_bundle(&hash_trytes);
        if !normalized.contains(&13) {
            to_return = hash_trytes.to_string();
            valid_to_sign = true;
        }else{
            tag = add(&tag, &[1]);           
        }       
    }
    (to_return.clone(), tag.trytes().unwrap()[0..27].to_string())
}



#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn print_ln_test() {
        let y = TEST_SEED.trits();
        let key_a = key(&TEST_SEED.trits(), 0, 1).unwrap();
        let key_b = key(&TEST_SEED.trits(), 1, 1).unwrap();
        //let key = iota_signing::key(&seed.trits(), index, security)?;
        let digests_a = digests(&key_a).unwrap();
        let address_a_trits = address(&digests_a).unwrap();
        let mut address_a = address_a_trits.trytes().unwrap();

        let digests_b = digests(&key_b).unwrap();
        let address_b_trits = address(&digests_b).unwrap();
        
        let mut address_b = address_b_trits.trytes().unwrap();
        let tw_bundle = calculate_normalized_timewarp_hash(&*address_b, TEST_TRUNK);
        let signed = sign_tw_hash(TEST_SEED, 0, &tw_bundle.0);
        
        let validated = validate_tw_signature(&address_a, &tw_bundle.0, &signed);
        println!("Success!! {}, {}", validated, signed);
        println!("Address to sign: {}", address_b);
        println!("tw bundle: {:?}", tw_bundle);
       // let normalized_bundle = 
        println!("{:?}", TEST_SEED.trits().trytes());

        //println!("{:?}", normalized_bundle.to_vec());
      
    }

}