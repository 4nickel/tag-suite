use std::hash::Hash;
use std::cmp::Eq;
use std::collections::HashSet;

pub fn conjunction<T: Hash + Eq>(sets: impl Iterator<Item=HashSet<T>>) -> HashSet<T> {
    let mut acc = HashSet::new();
    for set in sets.into_iter() { acc.extend(set.into_iter()) }
    acc
}

pub fn disjunction<T: Hash + Eq>(sets: impl Iterator<Item=HashSet<T>>) -> HashSet<T> {

    let mut acc = HashSet::new();
    let mut iterator = sets.into_iter();
    if let Some(set) = iterator.next() {
        acc.extend(set.into_iter());
    }
    while let Some(set) = iterator.next() {
        acc.retain(|v| set.contains(v));
    }
    acc
}
