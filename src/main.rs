extern crate nom;

use nom::*;
use std::fs::File;
use std::io::Read;
use std::env;
use std::mem::size_of;

#[derive(Debug)]
struct DataElement {
    tag_class: TagClass,
    constructed: bool,
    id: u32,
    data: Vec<u8>,
}

#[derive(Debug, PartialEq)]
enum TagClass {
    Universal,
    Application,
    Context,
    Private,
}

fn parse_identifier(input: &[u8]) -> IResult<&[u8], (TagClass, bool, u32)> {
    let tag_class = match (input[0] & 0xC0) >> 6 {
        0 => TagClass::Universal,
        1 => TagClass::Application,
        2 => TagClass::Context,
        3 => TagClass::Private,
        _ => unimplemented!(),
    };

    let constructed = input[0] & 0x20 == 0x20;

    if input[0] & 0x1f != 0x1f {
        let id = input[0] as u32 & 0x1f;
        return IResult::Done(&input[1..], (tag_class, constructed, id));
    } else {
        for i in 1..5 {
            if input[i] & 0x80 != 0x80 {
                let mut identifier: u32 = 0;
                for j in 1..=i {
                    let shift = (i - j) * 7;
                    identifier |= (input[j] as u32 & 0x7f) << shift;
                }
                return IResult::Done(&input[i..], (tag_class, constructed, identifier));
            }
        }
        return IResult::Error(ErrorKind::Custom(0));
    }
}

#[test]
fn test_parse_identifier() {
    let d = b"\x2a";
    let (_, r) = parse_identifier(d).unwrap();
    assert_eq!(r.0, TagClass::Universal);
    assert_eq!(r.1, true);
    assert_eq!(r.2, 0xa);

    let d = b"\xff\x2a";
    let (_, r) = parse_identifier(d).unwrap();
    assert_eq!(r.0, TagClass::Private);
    assert_eq!(r.1, true);
    assert_eq!(r.2, 0x2a);

    let d = b"\xff\x8a\x2a";
    let (_, r) = parse_identifier(d).unwrap();
    assert_eq!(r.0, TagClass::Private);
    assert_eq!(r.1, true);
    assert_eq!(r.2, 0x52a);
}

fn parse_length(input: &[u8]) -> IResult<&[u8], usize> {
    let mut length: usize = 0;
    let rest: &[u8];

    if input[0] & 0x80 != 0x80 {
        length = input[0] as usize & 0x7f;
        rest = &input[1..];
    } else {
        let length_length = input[0] as usize & 0x7f;
        if length_length > size_of::<usize>() {
            return IResult::Error(ErrorKind::Custom(0));
        }

        for i in 1..=length_length {
            length |= (input[i] as usize) << ((length_length - i) * 8);
        }
        rest = &input[length_length+1..];
    }
    return IResult::Done(rest, length);
}

#[test]
fn test_parse_length() {
    let d = b"\x82\x05\x63";
    let (rest, r) = parse_length(d).unwrap();
    assert_eq!(rest.len(), 0);
    assert_eq!(r, 1379);

    let d = b"\x09";
    let (rest, r) = parse_length(d).unwrap();
    assert_eq!(rest.len(), 0);
    assert_eq!(r, 9);
}

named!(parse_data_element<&[u8], DataElement>, do_parse!(
    id: parse_identifier >>
    data: length_bytes!(parse_length) >>
    (DataElement {
        id: id.2,
        tag_class: id.0,
        constructed: id.1,
        data: data.to_vec(),
    })
));

fn main() {
    let fname = env::args().last().unwrap();
    let mut file = File::open(fname).unwrap();
    let mut buf: Vec<u8> = Vec::with_capacity(1000);
    file.read_to_end(&mut buf);

    let r = parse_data_element(&buf[..]).unwrap();
    println!("{:?}", r);
}
