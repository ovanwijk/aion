
use crate::iota_signing::*;
use crate::iota_conversion::{Trinary,long_value};
use crate::iota_constants;
use crate::iota_utils;

use crate::iota_crypto::{Kerl, Sponge};
use crate::iota_model::Bundle;
use iota_constants::HASH_TRINARY_SIZE;
//use std::cmp;




pub fn sign_tw_hash(private_key:&Vec<i8>, tw_hash:&str) -> String {
    //let key_a = private_key;
    let normalized_hash = Bundle::normalized_bundle(&tw_hash);
    let signature = signature_fragment(&normalized_hash[0..27], &private_key[0..6561]).unwrap();
    signature.trytes().unwrap()
}

pub fn validate_tw_signature(pub_key:&str, tw_hash:&str, signature:&str) -> bool {
    validate_signatures(pub_key, &[String::from(signature)], tw_hash).unwrap()
}

pub fn increase_index(index:&str) -> String {
    iota_utils::trit_adder::add(&index.trits(), &[1]).trytes().expect("Trytes to work")
}

pub fn trytes_to_number(trytes:&str) -> i64 {
    long_value(&trytes.trits())
}
/**
 * Returns Key Trit vector and the corresponding public key trytes (address)
 */
pub fn generate_key_and_address(seed:&str, index:usize) -> (Vec<i8>, String) {
    //We only care about security 1 for this.
    let key_a = key(&seed.trits(), index, 1).unwrap();
       
    let digests_a = digests(&key_a).unwrap();
    let address_a_trits = address(&digests_a).unwrap();
    let mut address_a = address_a_trits.trytes().unwrap();

    (key_a, address_a)
}


pub fn timewarp_hash(address: &str, trunk:&str, branch:&str, tag:&str) -> String {
    let mut a = address.trits();
    a.append(&mut trunk.trits());
    a.append(&mut branch.trits());
  
    let padded_tag = format!("{}{}",tag, "999999999999999999999999999999999999999999999999999999");
    a.append(&mut padded_tag.trits());
    let mut curl = Kerl::default();
    let mut hash_trits = [0; HASH_TRINARY_SIZE];       
    let _l = curl.absorb(&a);        
    let _l = curl.squeeze(&mut hash_trits);
    let hash_trytes = hash_trits.trytes().unwrap();
    hash_trytes
}

pub fn calculate_normalized_timewarp_hash(address: &str, trunk:&str, branch:&str, index:&str, unique_id:&str) -> (String, String) {    
    let mut valid_to_sign = false;
    let mut to_return = String::from("");
    if index.len() != 5 {
        warn!("index {} must 9 trytes long", index);
    }
    if unique_id.len() != 9 {
        warn!("Unique id {} must 9 trytes long", unique_id);
    }

    //We include TW on the end of the tag because some libraries (including the rust one)
    //will copy the obsolute tag as the tag field if it is empty. Meaning the tag that actually
    //gets stored in the tangle might be a different one then used for normalizing alternate signing.
    let mut tag = ["99999999999", index, unique_id, "TW"].concat().trits();
    while !valid_to_sign {        
       let hash_trytes = timewarp_hash(address, trunk, branch, &tag.trytes().unwrap());
       let normalized = Bundle::normalized_bundle(&hash_trytes);
        if !normalized[0..27].contains(&13) { //we can make it lighter since we only use the first 27 trytes for signing (security 1)
            to_return = hash_trytes.to_string();
            valid_to_sign = true;
        }else{
            tag = iota_utils::trit_adder::add(&tag, &[1]);           
        }       
    }
    (to_return.clone(), tag.trytes().unwrap().to_string())
}



#[cfg(test)]
mod test {
    use super::*;


const TEST_SEED: &str =
        "IHDEENZYITYVYSPKAURUZAQKGVJEREFDJMYTANNXXGPZ9GJWTEOJJ9IPMXOGZNQLSNMFDSQOTZAEETUEA";

const TEST_TRUNK: &str =
        "BBBEENZYITYVYSPKAURUZAQKGVJEREFDJMYTANAAAGPZ9GJWTEOJJ9IPMXOGZNQLSNMFDSQOTZAEETUEA";



     #[test]
    fn manual() {
        let sig = "NOOGMSXTUCWYSXQESWTHPGRNVRIZEJLECVMYEZ9LHVPNADFVUJRTHFXXUHWZHQULKLWUTZQCGYWKCYACDSPTXGWRNWFYRAUKIFIIMNZNJ9GATAANKBUHCOXVOLFF9VQVYDBDFGFFAJDFEEBKSWLKISAUJWQFJWQVUXRETGVLDRCJJNISAHTJZKTWSF9MCTNNJGBLPYGSMQRLME9OAJVLNN9MINEZFYEMHGXQLXEFRZGTVCGJGFCOFAGOVGWVACRJWKWPZRITIMUXX9KTZTUXIXYTSPGPSWPXQPOHDPPEJDHSJEUDN9WYNUHQKMLGJ9OLFLXCVLCJV9ZDFUEXINBMIIPGXNIBJHSUVKBQJRBQSH9V9FTWEBNKVJIZVKKYISUCQKAL9EAEC9AORSBCLMMTWYNJYEYUQHJARJDOIRNKQQYPTYZRQEFDAZHAGLDB99NHHRRJ9WFBOMFLVYW9NHGESXRMBNWFK9AU9GG9VCLOR9HUZOHTGBLJYSGHQOIHMXHELLUEBFGNSXTKAPPPELROVSYECXOMKFRVNNMNDXHREOECIGCIGXUOUNX9VJRGBJNPKPHQESDTUJCGQQEVUTPIXFRMOLMAOBTAFSWIIPUKUUDSTGI99XHBNSVXWNDDBIDVQTAUEBTBNSASCTIHRSHEVMYXQPSEWTRZILEEPO9MRTJBYQOWIKDNKIDHMCKYDFFODQTJVICYO9DRAISYBPLI9OMSDIZOTAVMMNDSQVE9HEQJFVXVQBWIXOXGHUSQMTLZQGZZVYNSPSXPOWJRSXOPJHQQVFA9MESZEZAQFKNLJWECKYHBACSYF9KOIKYPV9XTF9YOGGUECRPLSMTVMR9HPHJZPCURDGLPJFGWIYSTOXSTNQDHPIEZYEEHGBBT9AUAFIDJGIFHGGS9NIBIMVRTURBAAYGILVMCNISQKSLAUFAPPXNBNPHZPWWMPKULXBVQAZZPDQVDGBBYYWZFMGINDVFHKPJU9IPBJMYVWIJUEUITWJJMLIXVZKPWUUBAUFNGNLJUFMPCUPTPETDJTYIGHWHFFEXIDXUSWSIQKZJKTFOGQWUDVYYVOYJRDBSHRLWTFORWRQAMIYFHOJEXEUUMBNWIOCPXXLWPXUUWAQZKMGROMDFXMZTUATKHGB9ZZ9GXJSRZBBFVSKTUKHVZAPYOKWHKJTKRAKDORQQKZYZOGJJXXFSTQDPUQXUTKCMOROWEFTJYNBNKOBQCATC9F9GQUIAJHGFUJWXDVKXZILRSNMRYYXVXNUUATLYZYMTITKXKRIARNCUENZC9QJMZGKNLUDUYFPMLSNTBLG9PRQRETYVBHSGTSSVKGTRCTGM9BXIXDBXWJDMHCVGXVSIAOJRJ9FPUJCDOKFWMW9UOOYKNGMGFMCBHIQSHQJQLLVGTAAM9SUSQVKUEKHXOQVFQKTWASQLIFOGWHWL9XDOPPIMGDLXMXLIQNDBCRIAQT9SSXAGVCJUBQFNEEARBNOVQJJZCNDZKSUBCTHMPPYGUVXROFOPJBWEQTSHAPQIPGVFFZNSHMPAIOQAGVOWVCSYPFQ9PBDWNIMSLZMWUHZTHSFUPQYCNGEDXVGKQIIBPPSTJ9SXEHURSCSTEAVKCAJSHVXMCGTNJBZP9ABWWQPHCAJYDNKWSVGNVOHWCRC9CTAFRULZHWTWCPIOXFXNTUICISDDETCQJSKEUKJGYVZYBXOJDHDDJANVHFPMOUVWWZPMYODDPXGPGTNKRQWZTVDDGFLGQOXBH9NHQIBUNPCGH9LEGPRRERGXQNSWTLYRYYDLNDRGSKSRVSGZBOPGVHEYZSJZPKXDDOPKTGFLVT9T9HUNVQQITKMJNGW9CZJDOEWZOCLRBVFDOYF9EFAFSSHCVZIWZFRR9ZDIMCCNP9MO9ALATRBUDKCLACIWCQZQBBWMBBPROBFFNZNHMZHNAOJDHAPOWPKTXBVNRKKDKRA99YDHYACSRT9TJZQXUTKW9NJSFWQRVKIMTSQJIPMDXIFAHCBTQLYHBPVULQQAMIWVVXAZVW9JFXNZDFOX9OVWBMIUKKJ9Y9JPAZDFWBYGQTRGOAWMBQIQN9DSQFAUILIWVMOVFPDJPYHSKSYVCGA9OJWHNOKSKNORNN9BZWLAFFOXZADSDE9JRHEMRMSGWPSFUOQIIBAVYVIVIFYTCYLVNXIREUCNIXSWMUPVHTLB";
        let mut address_validate = "BFERHMFIOKUWX9JWQNBDCDEYRXCLWVMVPGSTKWYSOKBHZYMOABXOKCVFUFGGUFKZDGKX9PZQFKHXMWNID";
        let mut address_me = "DIZNZKDGJKPYHHHAM9UKPJNATZNAXTSEVOHFNTCPSBKOCMVOGCJAOPPHHURLRPMIIOGJYEMWMLTRRSQD9";
        let trunk = "AUYZYT99HDRMBTSBPNTZUILBSSIUHIWHUZRORESCP9YUTT9LYZOYKRNQZUVZCN9JQTKLUNVAOYYQACQO9";
        let branch = "OMQLXTQVX9FPCNVBOEMKTQCXKTDYLVXO9LG9EMSKHOTZLJYXBOZIHJSDWPMTTGJSJJPVIZEOVGJWLNJU9";
        let tag = "GH9999999999999999999999999";


        let tw_hash = "PYXRFZZVRRLIBFYUPYLCUCBTFJSRIBLJ9CTPCPHCQRWWSYYVBBAZDCHXQNDFEJ9GJSQFIAUCLGPGVPVRX";
        let tw_hash1 = calculate_normalized_timewarp_hash(&address_me, &trunk, &branch,"99999", "999999999");
        let tw_hash = timewarp_hash(address_me, trunk, branch, tag);
        println!("{} - {} - {}", tw_hash, tw_hash1.0, tw_hash1.1);
        let validated = validate_tw_signature(address_validate, &tw_hash, sig);
        println!("Success!! {}", validated);
    }

    #[test]
    fn print_ln_test() {
        println!("Step 1");
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
        println!("Step 2");
        let tw_bundle = calculate_normalized_timewarp_hash(&*address_b, TEST_TRUNK, TEST_TRUNK,"99999", "999999999");
        println!("Step 3");
        let signed = sign_tw_hash(&key_a, &tw_bundle.0);
        println!("Step 4");
        
        let validated = validate_tw_signature(&address_a, &tw_bundle.0, &signed);
        println!("Success!! {}, {}", validated, signed);
        println!("Address to sign: {}", address_b);
        println!("tw bundle: {:?}", tw_bundle);
       // let normalized_bundle = 
        println!("{:?}", TEST_SEED.trits().trytes());

        //println!("{:?}", normalized_bundle.to_vec());
      
    }

}