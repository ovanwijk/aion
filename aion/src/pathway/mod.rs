const _E:u8 = 0b00000000u8;
const _T:u8 = 0b10000000u8;
const _B:u8 = 0b01000000u8;
const _Y:u8 = 0b11000000u8;

const STEPS_PER_FIELD:usize = 4; // 



#[derive(Clone, Debug)]
pub struct PathwayDescriptor {
    pub start:String,
    pub size:usize,
    pub fields:Vec<u8>
}


impl PathwayDescriptor {
    fn new(start:String) -> PathwayDescriptor {
        PathwayDescriptor {
            start: start,
            size: 0,
            fields: vec!()
        }
    }

    pub fn add_to_path(&mut self, what:u8) -> usize {
       
        if self.size % STEPS_PER_FIELD == 0 {
            println!("adding!");
            self.fields.push(0b00000000u8);
        }
        let edited = self.fields.last().expect("Last field should always be there");        
        let shifted = what >> (2 * (self.size % STEPS_PER_FIELD));
        let added = edited | shifted;
        println!("i: {} w:{:b} sh:{:b} ed:{:b} added:{:b}", 2 * (self.size % STEPS_PER_FIELD) ,what, shifted, edited, added);
        std::mem::replace(self.fields.last_mut().expect("Last field to be present"), added);
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

    pub fn from_vec(v:Vec<u8>, start:String) -> PathwayDescriptor {
        PathwayDescriptor {
            start: start,
            size: v.len() * STEPS_PER_FIELD,
            fields: v
        }
    }
}

pub struct PathwayIterator {
    descriptor: PathwayDescriptor,
    index: usize
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
    #[test]
    fn adding() {

        let mut a = PathwayDescriptor::new("TEST".to_string());
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
}
