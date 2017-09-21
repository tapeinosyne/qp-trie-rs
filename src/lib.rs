#[macro_use]
extern crate debug_unreachable;
extern crate unreachable;

#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

#[cfg(test)]
extern crate rand;
#[cfg(test)]
extern crate serde_json;


#[cfg(feature = "serde")]
mod serialization;

mod entry;
mod iter;
mod node;
mod sparse;
mod subtrie;
mod trie;
mod util;

pub mod wrapper;

pub use entry::{Entry, OccupiedEntry, VacantEntry};
pub use iter::{Iter, IterMut, IntoIter};
pub use trie::{Trie, Break};
pub use subtrie::SubTrie;


#[cfg(test)]
mod test {
    use super::*;

    use std::collections::HashMap;

    use rand::Rng;
    use serde_json;
    use quickcheck::TestResult;

    use util::nybble_index;

    quickcheck! {
        fn nybble(nybs: Vec<u8>) -> TestResult {
            for &nyb in &nybs {
                if nyb > 15 {
                    return TestResult::discard();
                }
            }

            let mut bytes = Vec::new();

            for chunk in nybs.chunks(2) {
                if chunk.len() == 2 {
                    bytes.push(chunk[0] | (chunk[1] << 4));
                } else {
                    bytes.push(chunk[0]);
                }
            }

            for (i, nyb) in nybs.into_iter().enumerate() {
                assert_eq!(nyb + 1, nybble_index(i, &bytes));
            }

            TestResult::passed()
        }

        fn insert_and_get(elts: Vec<(u8, u64)>) -> bool {
            let mut elts = elts;
            let mut rng = rand::thread_rng();
            elts.sort_by_key(|e| e.0);
            elts.dedup_by_key(|e| e.0);
            rng.shuffle(&mut elts);

            let hashmap: HashMap<u8, u64> = elts.iter().cloned().collect();
            let trie = {
                let mut trie = Trie::<[u8; 1], u64>::new();

                for (i, (b, s)) in elts.into_iter().enumerate() {
                    assert_eq!(trie.count(), i);
                    trie.insert([b], s);
                }

                trie
            };


            for (&key, &value) in hashmap.iter() {
                if trie.get(&[key]) != Some(&value) {
                    return false;
                }
            }

            for (&key, &value) in trie.iter() {
                if hashmap[&key[0]] != value {
                    return false;
                }
            }

            return true;
        }

        fn insert_and_remove(elts: Vec<(Vec<u8>, Option<u64>)>) -> bool {
            let mut hashmap = HashMap::new();
            let mut trie = Trie::new();

            for &(ref k, v_opt) in &elts {
                match v_opt {
                    Some(v) => {
                        hashmap.insert(k.as_ref(), v);
                        trie.insert(k.as_ref(), v);
                    }
                    None => {
                        hashmap.remove(&k.as_ref());
                        trie.remove(&k.as_ref());
                    },
                }
            }

            let collected: HashMap<&[u8], u64> = trie.into_iter().collect();

            hashmap == collected
        }

        fn prefix_sets(prefix: Vec<u8>, elts: Vec<(Vec<u8>, u64)>) -> bool {
            let mut trie = Trie::new();

            for &(ref k, v) in elts.iter() {
                trie.insert(&k[..], v);
            }

            let filtered: HashMap<&[u8], u64> = trie.iter().filter_map(|(&key, &val)| if key.starts_with(&prefix[..]) { Some((key, val)) } else { None }).collect();
            let prefixed: HashMap<&[u8], u64> = trie.remove_prefix(&prefix[..]).into_iter().collect();

            filtered == prefixed
        }

        fn prefix_sets_ref(prefix: Vec<u8>, elts: Vec<(Vec<u8>, u64)>) -> bool {
            let mut trie = Trie::new();

            for &(ref k, v) in elts.iter() {
                trie.insert(&k[..], v);
            }

            let filtered: HashMap<&[u8], u64> = trie.iter().filter_map(|(&key, &val)| if key.starts_with(&prefix[..]) { Some((key, val)) } else { None }).collect();
            let prefixed: HashMap<&[u8], u64> = trie.iter_prefix(&prefix[..]).map(|(&key, &val)| (key, val)).collect();
            let subtried: HashMap<&[u8], u64> = trie.subtrie(&prefix[..]).iter().map(|(&key, &val)| (key, val)).collect();

            filtered == prefixed && filtered == subtried
        }

        fn prefix_sets_mut(prefix: Vec<u8>, elts: Vec<(Vec<u8>, u64)>) -> bool {
            let mut trie = Trie::new();

            for &(ref k, v) in elts.iter() {
                trie.insert(&k[..], v);
            }

            let filtered: HashMap<&[u8], u64> = trie.iter_mut().filter_map(|(&key, &mut val)| if key.starts_with(&prefix[..]) { Some((key, val)) } else { None }).collect();
            let prefixed: HashMap<&[u8], u64> = trie.iter_prefix_mut(&prefix[..]).map(|(&key, &mut val)| (key, val)).collect();

            filtered == prefixed
        }

        fn entry_insert_and_remove(elts: Vec<(Vec<u8>, Option<u64>)>) -> bool {
            let mut hashmap = HashMap::new();
            let mut trie = Trie::new();

            for &(ref k, v_opt) in &elts {
                match v_opt {
                    Some(v) => {
                        hashmap.insert(k.as_ref(), v);
                    
                        match trie.entry(k.as_ref()) {
                            Entry::Occupied(mut occupied) => { occupied.insert(v); }
                            Entry::Vacant(vacant) => { vacant.insert(v); }
                        }
                    }
                    None => {
                        hashmap.remove(&k.as_ref());

                        match trie.entry(k.as_ref()) {
                            Entry::Occupied(occupied) => { occupied.remove(); },
                            Entry::Vacant(..) => {},
                        }
                    },
                }
            }

            let collected: HashMap<&[u8], u64> = trie.into_iter().collect();

            hashmap == collected
        }

        fn longest_common_prefix(boolfix: Vec<bool>, boolts: Vec<(Vec<bool>, u64)>) -> TestResult {
            let prefix = boolfix.into_iter().map(|b| if b { 1 } else { 0 }).collect::<Vec<u8>>();
            let elts = boolts.into_iter().map(|(key, val)| (key.into_iter().map(|b| if b { 1 } else { 0 }).collect::<Vec<u8>>(), val)).collect::<Vec<(Vec<u8>, u64)>>();
            
            let mut trie = Trie::new();

            for &(ref k, v) in elts.iter() {
                trie.insert(&k[..], v);
            }

            let lcp = elts.iter().fold(&[][..], |lcp, &(ref k, _)| {
                let mut i = 0;

                for (j, (b, c)) in k.iter().cloned().zip(prefix.iter().cloned()).enumerate() {
                    if b != c {
                        break;
                    }

                    i = j + 1;
                }

                if i >= lcp.len() {
                    &prefix[..i]
                } else {
                    lcp
                }
            });

            TestResult::from_bool(lcp == trie.longest_common_prefix(prefix.as_slice()))
        }

        #[cfg(feature = "serde")]
        fn serialize(kvs: Vec<(String, usize)>) -> bool {
            use wrapper::BString;

            let original: Trie<BString, _> = kvs.into_iter().map(|(k, v)| (k.into(), v)).collect();
            let serialized = serde_json::to_vec(&original).unwrap();
            let deserialized: Trie<BString, usize> = serde_json::from_slice(&serialized).unwrap();

            deserialized == original
        }

        #[cfg(feature = "serde")]
        fn serialize_emptied(kvs: Vec<(String, usize)>) -> bool {
            use wrapper::BString;

            let wrap = kvs.clone().into_iter().map(|(k, v)| (BString::from(k), v));
            let mut trie: Trie<BString, usize> = wrap.collect();
            
            for (k, _) in kvs {
                trie.remove(&BString::from(k));
            }

            let serialized = serde_json::to_vec(&trie).unwrap();
            let deserialized: Trie<BString, usize> = serde_json::from_slice(&serialized).unwrap();

            deserialized == Trie::new()
        }
    }


    fn entry_insert_and_remove_regression(elts: Vec<(Vec<u8>, Option<u64>)>) -> bool {
        let mut hashmap = HashMap::new();
        let mut trie = Trie::new();

        for &(ref k, v_opt) in &elts {
            match v_opt {
                Some(v) => {
                    hashmap.insert(k.as_ref(), v);

                    match trie.entry(k.as_ref()) {
                        Entry::Occupied(mut occupied) => {
                            occupied.insert(v);
                        }
                        Entry::Vacant(vacant) => {
                            vacant.insert(v);
                        }
                    }
                }
                None => {
                    hashmap.remove(&k.as_ref());

                    match trie.entry(k.as_ref()) {
                        Entry::Occupied(occupied) => {
                            occupied.remove();
                        }
                        Entry::Vacant(..) => {}
                    }
                }
            }
        }

        let collected: HashMap<&[u8], u64> = trie.into_iter().collect();

        hashmap == collected
    }


    #[test]
    fn entry_insert_and_remove_1() {
        entry_insert_and_remove_regression(vec![
            (vec![83], Some(0)),
            (vec![83, 0], Some(0)),
            (vec![35], Some(0)),
        ]);
    }


    #[test]
    fn entry_insert_and_remove_2() {
        entry_insert_and_remove_regression(vec![
            (vec![30], Some(0)),
            (vec![30, 0], Some(0)),
            (vec![13], Some(0)),
        ]);
    }


    fn prefix_sets_regression(prefix: Vec<u8>, elts: Vec<(Vec<u8>, u64)>) {
        let mut trie = Trie::new();

        for &(ref k, v) in elts.iter() {
            trie.insert(&k[..], v);
        }

        let filtered: HashMap<&[u8], u64> = trie.iter()
            .filter_map(|(&key, &val)| if key.starts_with(&prefix[..]) {
                Some((key, val))
            } else {
                None
            })
            .collect();
        let prefixed: HashMap<&[u8], u64> = trie.remove_prefix(&prefix[..]).into_iter().collect();

        assert_eq!(filtered, prefixed);
    }


    #[test]
    fn prefix_sets_1() {
        prefix_sets_regression(vec![], vec![(vec![], 0), (vec![0], 0)]);
    }


    fn insert_and_remove_regression(elts: Vec<(Vec<u8>, Option<u64>)>) {
        let mut hashmap = HashMap::new();
        let mut trie = Trie::new();

        for &(ref k, v_opt) in &elts {
            match v_opt {
                Some(v) => {
                    hashmap.insert(k.as_ref(), v);
                    trie.insert(k.as_ref(), v);
                }
                None => {
                    hashmap.remove(&k.as_ref());
                    trie.remove(&k.as_ref());
                }
            }
        }

        let collected: HashMap<&[u8], u64> = trie.into_iter().collect();

        assert_eq!(hashmap, collected);
    }


    #[test]
    fn insert_and_remove_1() {
        insert_and_remove_regression(vec![
            (vec![], Some(0)),
            (vec![46], Some(0)),
            (vec![62], None),
        ]);
    }


    fn insert_and_get_vec(elts: Vec<(u8, u64)>) {
        let hashmap: HashMap<u8, u64> = elts.iter().cloned().collect();
        let trie = {
            let mut trie = Trie::<[u8; 1], u64>::new();

            for (i, (b, s)) in elts.into_iter().enumerate() {
                assert_eq!(trie.count(), i);
                trie.insert([b], s);
            }

            trie
        };


        for (key, value) in hashmap {
            assert_eq!(
                trie.get(&[key]),
                Some(&value),
                "Sad trie: {:?}",
                trie,
            );
        }
    }

    #[test]
    fn insert_and_get_1() {
        insert_and_get_vec(vec![(17, 0), (0, 0), (16, 0), (18, 0)]);
    }

    #[test]
    fn insert_and_get_2() {
        insert_and_get_vec(vec![(5, 0), (0, 5), (1, 13), (49, 31)]);
    }

    #[test]
    fn insert_and_get_3() {
        insert_and_get_vec(vec![(57, 0), (41, 0), (0, 0), (89, 0)]);
    }

    #[test]
    fn insert_and_get_4() {
        insert_and_get_vec(vec![(3, 0), (35, 0), (0, 2), (13, 0)]);
    }

    #[test]
    fn insert_and_get_5() {
        insert_and_get_vec(vec![(0, 0), (32, 9), (87, 5), (89, 26)]);
    }


    #[test]
    fn longest_common_prefix_simple() {
        use wrapper::{BString, BStr};

        let mut trie = Trie::<BString, u32>::new();

        trie.insert("z".into(), 2);
        trie.insert("aba".into(), 5);
        trie.insert("abb".into(), 6);
        trie.insert("abc".into(), 50);

        let ab_sum = trie.iter_prefix(trie.longest_common_prefix(AsRef::<BStr>::as_ref("abd"))).fold(0, |acc, (_, &v)| {
            println!("Iterating over child: {:?}", v);

            acc + v
        });

        println!("{}", ab_sum);
        assert_eq!(ab_sum, 5 + 6 + 50);
    }


    #[test]
    fn longest_common_prefix_complex() {
        use wrapper::{BString, BStr};

        let mut trie = Trie::<BString, u32>::new();

        trie.insert("z".into(), 2);
        trie.insert("aba".into(), 5);
        trie.insert("abb".into(), 6);
        trie.insert("abc".into(), 50);

        let ab_sum = trie.iter_prefix(trie.longest_common_prefix(AsRef::<BStr>::as_ref("abz"))).fold(0, |acc, (_, &v)| {
            println!("Iterating over child: {:?}", v);

            acc + v
        });

        println!("{}", ab_sum);
        assert_eq!(ab_sum, 5 + 6 + 50);
    }


    #[test]
    #[cfg(feature = "serde")]
    fn serialize_pathological_branching() {
        use wrapper::BString;

        let kvs = (0 .. 64).map(|length| {
            let seq = vec![0; length];
            let k = String::from_utf8(seq).unwrap();
            (BString::from(k), 0)
        });

        let original : Trie<BString, u8> = kvs.collect();
        let serialized = serde_json::to_vec(&original).unwrap();
        let deserialized: Trie<_, _> = serde_json::from_slice(&serialized).unwrap();

        assert_eq!(deserialized, original);
    }
}
