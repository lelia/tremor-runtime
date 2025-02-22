// Copyright 2020-2021, The Tremor Team
//
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Influx line protocol decoder to `simd_json::Value`
//!
//! The translate is done to the following schema:
//!
//! ```json
//! {
//!    "measurement": "<name>",
//!    "tags": {
//!      "tag1": "<value>"
//!    },
//!    "fields": {
//!      "field1": 123
//!    },
//!    "timestamp": 456
//! }
//! ```

#![deny(missing_docs)]
#![recursion_limit = "1024"]
#![deny(
    clippy::all,
    clippy::unwrap_used,
    clippy::unnecessary_unwrap,
    clippy::pedantic,
    clippy::mod_module_files
)]
// TODO: remove this when https://github.com/rust-lang/rust-clippy/issues/9076 is fixed
#![allow(clippy::trait_duplication_in_bounds)]

mod decoder;
mod encoder;
/// Errors
pub mod errors;

pub use decoder::decode;
pub use encoder::encode;
pub use errors::*;

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use simd_json::{json, BorrowedValue};

    #[test]
    fn unparse_test() {
        let s = "weather,location=us-midwest temperature=82 1465839830100400200";
        let d = decode(s, 0)
            .expect("failed to parse")
            .expect("failed to parse");
        // This is a bit ugly but to make a sensible comparison we got to convert the data
        // from an object to json to an object
        let j: BorrowedValue = d;
        let e: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {"location": "us-midwest"},
            "fields": {"temperature": 82.0},
            "timestamp": 1_465_839_830_100_400_200_i64
        })
        .into();

        assert_eq!(e, j)
    }

    #[test]
    fn unparse_empty() {
        let s = "";
        let d: Option<BorrowedValue> = decode(s, 0).expect("failed to parse");
        assert!(d.is_none());
        let s = "  ";
        let d: Option<BorrowedValue> = decode(s, 0).expect("failed to parse");
        assert!(d.is_none());
        let s = "  \n";
        let d: Option<BorrowedValue> = decode(s, 0).expect("failed to parse");
        assert!(d.is_none());
        let s = " \t \n";
        let d: Option<BorrowedValue> = decode(s, 0).expect("failed to parse");
        assert!(d.is_none());
    }

    #[test]
    fn unparse_comment() {
        let s = "# bla";
        let d: Option<BorrowedValue> = decode(s, 0).expect("failed to parse");
        assert!(d.is_none());
        let s = "  # bla";
        let d: Option<BorrowedValue> = decode(s, 0).expect("failed to parse");
        assert!(d.is_none());
        let s = " \t \n# bla";
        let d: Option<BorrowedValue> = decode(s, 0).expect("failed to parse");
        assert!(d.is_none());
    }

    #[test]
    fn parse_simple() {
        let s = "weather,location=us-midwest temperature=82 1465839830100400200";
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {
                "location": "us-midwest"
            },
            "fields": {
                "temperature": 82.0
            },
            "timestamp": 1_465_839_830_100_400_200_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }

    #[test]
    fn parse_simple2() {
        let s = "weather,location=us-midwest,season=summer temperature=82 1465839830100400200";
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {
                "location": "us-midwest",
                "season": "summer"
            },
            "fields": {
                "temperature": 82.0
            },
            "timestamp": 1_465_839_830_100_400_200_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }

    #[test]
    fn parse_example() {
        let mut s = br#"{"measurement":"swap","tags":{"host":"56a6f1b85709","window":"10secs"},"fields":{"count_free":2,"min_free":2139095040,"max_free":2147483647,"mean_free":2143289344.0,"stdev_free":0.0,"var_free":0.0,"p50_free":2147483647,"p90_free":2147483647,"p99_free":2147483647,"p99.9_free":2147483647},"timestamp":1465839830100400200}"#.to_vec();
        let v = simd_json::borrowed::to_value(s.as_mut_slice()).unwrap();
        encode(&v).unwrap();
    }

    #[test]
    fn parse_simple3() {
        let s =
            "weather,location=us-midwest temperature=82,bug_concentration=98 1465839830100400200";
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {
                "location": "us-midwest"
            },
            "fields": {
                "temperature": 82.0,
                "bug_concentration": 98.0,

            },
            "timestamp": 1_465_839_830_100_400_200_u64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }

    #[test]
    fn parse_no_timestamp() {
        let s = "weather temperature=82i";
        let parsed = decode(s, 1_465_839_830_100_400_200_u64).expect("failed to parse");
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {},
            "fields": {
                "temperature": 82
            },
            "timestamp": 1_465_839_830_100_400_200_i64,
        })
        .into();
        assert_eq!(Some(r), parsed);
    }

    #[test]
    fn parse_float_value() {
        let s = "weather temperature=82 1465839830100400200";
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {},
            "fields": {
                "temperature": 82.0
            },
            "timestamp": 1_465_839_830_100_400_200_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }

    #[test]
    fn parse_int_value() {
        let s = "weather temperature=82i 1465839830100400200";
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {},
            "fields": {
                "temperature": 82
            },
            "timestamp": 1_465_839_830_100_400_200_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }

    #[test]
    fn parse_str_value() {
        let s = "weather,location=us-midwest temperature=\"too warm\" 1465839830100400200";
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {
                "location": "us-midwest"
            },
            "fields": {
                "temperature": "too warm"
            },
            "timestamp": 1_465_839_830_100_400_200_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }
    #[test]
    fn parse_true_value() {
        let sarr = &[
            "weather,location=us-midwest too_hot=true 1465839830100400200",
            "weather,location=us-midwest too_hot=True 1465839830100400200",
            "weather,location=us-midwest too_hot=TRUE 1465839830100400200",
            "weather,location=us-midwest too_hot=t 1465839830100400200",
            "weather,location=us-midwest too_hot=T 1465839830100400200",
        ];
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {
                "location": "us-midwest"
            },
            "fields": {
                "too_hot": true
            },
            "timestamp": 1_465_839_830_100_400_200_i64,
        })
        .into();
        for s in sarr {
            assert_eq!(Ok(Some(r.clone())), decode(s, 0));
        }
    }
    #[test]
    fn parse_false_value() {
        let sarr = &[
            "weather,location=us-midwest too_hot=false 1465839830100400200",
            "weather,location=us-midwest too_hot=False 1465839830100400200",
            "weather,location=us-midwest too_hot=FALSE 1465839830100400200",
            "weather,location=us-midwest too_hot=f 1465839830100400200",
            "weather,location=us-midwest too_hot=F 1465839830100400200",
        ];
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {
                "location": "us-midwest"
            },
            "fields": {
                "too_hot": false
            },
            "timestamp": 1_465_839_830_100_400_200_i64,
        })
        .into();
        for s in sarr {
            assert_eq!(Ok(Some(r.clone())), decode(s, 0));
        }
    }
    #[test]
    fn parse_escape01() {
        let s = "weather,location=us\\,midwest temperature=82 1465839830100400200";
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {
                "location": "us,midwest"
            },
            "fields": {
                "temperature": 82.0
            },
            "timestamp": 1_465_839_830_100_400_200_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }

    #[test]
    fn parse_escape02() {
        let s = "weather,location=us-midwest temp\\=rature=82 1465839830100400200";
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {
                "location": "us-midwest"
            },
            "fields": {
                "temp=rature": 82.0
            },
            "timestamp": 1_465_839_830_100_400_200_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }
    #[test]
    fn parse_escape03() {
        let s = "weather,location\\ place=us-midwest temperature=82 1465839830100400200";
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {
                "location place": "us-midwest"
            },
            "fields": {
                "temperature": 82.0
            },
            "timestamp": 1_465_839_830_100_400_200_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }

    #[test]
    fn parse_escape04() {
        let s = "wea\\,ther,location=us-midwest temperature=82 1465839830100400200";
        let r: BorrowedValue = json!({
            "measurement": "wea,ther",
            "tags": {
                "location": "us-midwest"
            },
            "fields": {
                "temperature": 82.0
            },
            "timestamp": 1_465_839_830_100_400_200_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }
    #[test]
    fn parse_escape05() {
        let s = "wea\\ ther,location=us-midwest temperature=82 1465839830100400200";
        let r: BorrowedValue = json!({
            "measurement": "wea ther",
            "tags": {
                "location": "us-midwest"
            },
            "fields": {
                "temperature": 82.0
            },
            "timestamp": 1_465_839_830_100_400_200_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }

    #[test]
    fn parse_escape06() {
        let s = r#"weather,location=us-midwest temperature="too\"hot\"" 1465839830100400200"#;
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {
                "location": "us-midwest"
            },
            "fields": {
                "temperature": r#"too"hot""#
            },
            "timestamp": 1_465_839_830_100_400_200_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }

    #[test]
    fn parse_escape07() {
        let s = "weather,location=us-midwest temperature_str=\"too hot/cold\" 1465839830100400201";
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {
                "location": "us-midwest"
            },
            "fields": {
                "temperature_str": "too hot/cold"
            },
            "timestamp": 1_465_839_830_100_400_201_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0));
    }

    #[test]
    fn parse_escape08() {
        let s = "weather,location=us-midwest temperature_str=\"too hot\\cold\" 1465839830100400202";
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {
                "location": "us-midwest"
            },
            "fields": {
                "temperature_str": "too hot\\cold"
            },
            "timestamp": 1_465_839_830_100_400_202_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }

    #[test]
    fn parse_escape09() {
        let s =
            "weather,location=us-midwest temperature_str=\"too hot\\\\cold\" 1465839830100400203";
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {
                "location": "us-midwest"
            },
            "fields": {
                "temperature_str": "too hot\\cold"
            },
            "timestamp": 1_465_839_830_100_400_203_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }

    #[test]
    fn parse_escape10() {
        let s =
            "weather,location=us-midwest temperature_str=\"too hot\\\\\\cold\" 1465839830100400204";
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {
                "location": "us-midwest"
            },
            "fields": {
                "temperature_str": "too hot\\\\cold"
            },
            "timestamp": 1_465_839_830_100_400_204_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }
    #[test]
    fn parse_escape11() {
        let s = "weather,location=us-midwest temperature_str=\"too hot\\\\\\\\cold\" 1465839830100400205";
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {


                "location": "us-midwest"
            },
            "fields": {
                "temperature_str": "too hot\\\\cold"
            },
            "timestamp": 1_465_839_830_100_400_205_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }
    #[test]
    fn parse_escape12() {
        let s = "weather,location=us-midwest temperature_str=\"too hot\\\\\\\\\\cold\" 1465839830100400206";
        let r: BorrowedValue = json!({
            "measurement": "weather",
            "tags": {
                "location": "us-midwest"
            },
            "fields": {
                "temperature_str": "too hot\\\\\\cold"
            },
            "timestamp": 1_465_839_830_100_400_206_i64,
        })
        .into();
        assert_eq!(Ok(Some(r)), decode(s, 0))
    }
}
