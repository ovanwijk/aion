use serde::{Serialize, Deserialize};
use crate::base64;
pub const _E:u8 = 0b00000000u8;
pub const _T:u8 = 0b10000000u8;
pub const _B:u8 = 0b01000000u8;
pub const _Y:u8 = 0b11000000u8;

const STEPS_PER_FIELD:usize = 4; // 



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathwayDescriptor {
    pub size:usize,
    pub tx_count:usize,
    #[serde(with = "base64")]
    pub fields:Box<Vec<u8>>
}



// impl IntoIterator for PathwayDescriptor {
//     type Item = u8;
//     type IntoIter = std::vec::IntoIter<Self::Item>;
   
//     fn into_iter(self) -> PathwayIterator{
//         PathwayIterator{
//             descriptor:self,
//             index: 0
//         }
//     }
// }


impl PathwayDescriptor {

    // pub fn hash(pathway:PathwayDescriptor, extra: String) -> String {

    // }

    pub fn new() -> PathwayDescriptor {
        PathwayDescriptor {
            size: 0,
            tx_count: 0,
            fields: Box::new(vec!())
        }
    }

    pub fn add_to_path(&mut self, what:u8) -> usize {
       
        if self.size % STEPS_PER_FIELD == 0 {
            self.fields.push(0b00000000u8);
        }
        let edited = self.fields.last().expect("Last field should always be there");        
        let shifted = what >> (2 * (self.size % STEPS_PER_FIELD));
        let added = edited | shifted;
        //println!("i: {} w:{:b} sh:{:b} ed:{:b} added:{:b}", 2 * (self.size % STEPS_PER_FIELD) ,what, shifted, edited, added);
        std::mem::replace(self.fields.last_mut().expect("Last field to be present"), added);
        if what == _Y {
            self.tx_count += 2;
        }else {
            self.tx_count += 1;
        }
        self.size += 1;
        self.size
    }

    pub fn get_at_index(&self, index:usize) -> Option<u8> {
        if index >= self.size {
            None
        }else{
            Some((self.fields[index/STEPS_PER_FIELD] << 2*(index % STEPS_PER_FIELD)) & 0b11000000u8)
        }
    }

    // pub fn from_vec(v:Vec<u8>) -> PathwayDescriptor {
    //     PathwayDescriptor {
    //         size: v.len() * STEPS_PER_FIELD,
    //         fields: Box::new(v),
    //         tx_count:0
    //     }
    // }

    pub fn format(&self) -> String {
        let mut result = String::from("");
        for f in self.fields.clone().into_iter() {
            result = format!("{}|{:b}", result, f);
        }
        result
    }

    pub fn extend(&mut self, pathway:PathwayDescriptor) {
        self.size -= 1;
        self.tx_count -= 1;
        //self.add_to_path(if trunk_or_branch {_T}else{_B});
        let iterator = PathwayIterator{
            descriptor:pathway,
            index: 0
        };
        for step in iterator {
            self.add_to_path(step);
        }
        //for what in pathway.
    }
}

pub struct PathwayIterator {
    pub descriptor: PathwayDescriptor,
    pub index: usize
}



impl Iterator for PathwayIterator {
    type Item = u8;
   
    fn next(&mut self) -> Option<u8> {
        let r = self.descriptor.get_at_index(self.index);
        self.index += 1;
        r
    }
}





#[cfg(test)]
mod test {
    use super::*;
    use crate::iota_api::PathfindingResult;
    #[test]
    fn adding() {

        let mut a = PathwayDescriptor::new();
        a.add_to_path(_Y);
        a.add_to_path(_Y);
        a.add_to_path(_E);
        a.add_to_path(_T);
        a.add_to_path(_T);
        a.add_to_path(_E);
        a.add_to_path(_B);
        a.add_to_path(_Y);

        println!("{:b}, {:b}", a.fields[0], a.fields[1]);
        assert_eq!(a.fields[0], 0b11110010);
        assert_eq!(a.get_at_index(0).unwrap(), _Y);
        assert_eq!(a.get_at_index(2).unwrap(), _E);
        assert_eq!(a.get_at_index(6).unwrap(), _B);
    }

    #[test]
    fn from_result() {
        //let txt = "{\"txIDs\":[\"QCJTVNA9UDKRSYHLMYQ9ZWQATRKMRXGJIHZ9MKBSWQXDBCUQDTTGVOMZIQCWMJDHWJYOBOCIZQBMGPEE9\",\"OCCEA9S9CIVV9PJRVHKRWUDQCXTUKJCOPSFHKNAENHXBRSZMJXYZJGBULNZTLSRQHECORTU9GUYRBIWP9\",\"URKDZNAMZXLRSYDZQO9OAKM9JOFRPDBMLTYCJEFPFQKTUHBVGTXGMDXGELIXWXZLCKDVAFWOOCYFZGAT9\",\"SIVSQVIW9LBALMAYBQR9XVJAIYNCCTITFHUIAURZUPMBLTRQHJXBUSTJC9FCNTLBLOXRNNVEENPXKOOP9\",\"BWPKUBJJQICGHWOHVNV9HIDVUBKVBBEJIVQ9NDKDTV9A9LQBGMWQLXSRHFYOHBCEPFLYQQJQMYAFZXOR9\",\"IQJQRPPYYNBQUGSCJZPRCQMHBIBSDAYMXDWRB9KKUKHSXVCPRCCLP9B9PKYDXJEKDTOSKJSXUUPIAVTF9\",\"BKUXYSOHJRRDZBPFTYEFYSJJIIBHCPMLJQKVTRZROIJZGKPLD9NIMRUDQQMKRYJXRGLIRXGMJCGZAORP9\",\"GTSBSJTYHQBBARNKWBVNMPBOQXLMELWXAMACOQIJ9ZUKEHGRUGBWOMNEOYPERWMUYCKADYZLPQRYVRMS9\",\"GVGDNQEJWFJCBYIEKXKEVCDZKINNWFZDQOCSH9PDMPTGPSE9VHSVBFNCUZQSMDPXFQNJXTJXSKZJEEGJ9\",\"JAYMCLVBBVXZKPJXJBX9VWPDMYCPYKFRJZASYQDVNCIXU9BODQAHGJLAEQAWVLPAECISZOCTGFUKNV9N9\",\"UWWGIVWUEKRCEUQLCUXHPJMHTCDHKMMFDHMXGXNXBTEOIMAGVXAEETYEEJDNHYNVUU9QJ9MIRFQG9KWZ9\",\"GLBUXXC9CGGWBEBETWLERWKQLFSUBPUJTRADGUDYQQAUGZRFVWN9KNOUXSPVKDZHGTXFLPVMFGTBKGYY9\",\"QLSIGGAKZMXELNGRF9AVCGZNATWKKUKWCK9HWIKX9LMHCIHFSQBOTTBIDZSPQSXRWSLLAHDCDA9X9KDV9\",\"SYZBKYKAKYAMJJWXMZVVMRRCQKTGXUQVYQI9SIOTYMLGEKSIXOOLKEDLXMERFVQISGSIVVMFIIAV9FSB9\",\"FATI9CZLVWMDJYQEFGUGUUTPDIQVEJQJTNYFHQACJYPSKBLGITXZPIMZEQKXRK9UUAUICTPUNFS9HPMR9\",\"ASWWSGYTPJTKUMKRGEXHXFBIEWCJTVCHFPQTGGMAHAWOEETOJIXMFCYXULXYLCBLGGQEK9HXOBPORFZG9\",\"PC9FEOMVLCEDUCMYRXOOSONDZXNWRBCCZDRFOMBOQNJSWODNCXFYFDFDQDZOECJFMSBAFHFZBBDI9GBS9\",\"G9JHGMPOLZCDHT9GBDSDRVNFCVTVDGMF9NPQIOW9WCKB9SKUXGMZDWOAUYQSEQRBZKMKCPOPNMJMRCZV9\",\"ORJYXQRXQXJJUFZ9CCLIVGPQKOKFADMCOPIAXLGYFOFTUKPKPYTKXCDEBFWBKPDTRQSLFXROFIGHFPTZ9\",\"SITLMLYQCNUGFWTSVIXK9ZHGZVDVOJTKVCVRKWWCEOHWLLCUMITTLCGJZIAIGBKKACTMAIEGNEBTARWW9\",\"DTFTXZYSMVLJDHSQJYVWOXIIDPPKYCQYSG9EPW9FVKBPXTUUXLET9PJT9XR9ZQYHEKSWHJ9EETHQVUNA9\",\"NJECHWXLKGYAURPDHMYSGWTDLBUKN9ELNGLGPRTPDX9QGBCXYYKFXSKQTXUDWKKCAFCKOUALAVLVTLMM9\",\"ZXV9XDTIGBVEAIJGKUIOMMFWFWKZDGERMTOAFMBJAPVQMCWMF9NUKGUSZUTGXMDUJPPWLVBZKXJNTHJX9\"],\"branches\":[[3,2],[5,4],[6,5],[7,6],[9,8],[10,9],[14,13],[15,14],[16,15],[17,16],[19,18],[22,21]],\"trunks\":[[1,0],[2,1],[4,3],[8,7],[11,10],[12,11],[13,12],[18,17],[20,19],[21,20]],\"duration\":5}";
        let txt = std::fs::read_to_string("./test.json").unwrap();
        let a:PathfindingResult = serde_json::from_str(&txt).expect("Json parsing to work");
        println!("{:?}", a);
        let pathway_desc = a.to_pathway(String::from("BVVONDGAUWEBPTAXKTVNRQEWWAMSOMRCUIDUWXVVWDVEGVHDVHOYADLJGKFCRZVADTDXAGTURUMWLLKK9"));
        println!("Test: {}", pathway_desc.format());
    }

   
}
