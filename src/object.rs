use crate as tg;
use std::collections::BTreeMap;

pub type Object = BTreeMap<String, tg::Value>;

crate::value!(Object);
