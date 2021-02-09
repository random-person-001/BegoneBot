// This whole thing maybe should have been done with serde but uh the deed is done here

/// https://doc.rust-lang.org/std/vec/struct.Vec.html#examples-4
///
///
/// This takes a vec of u8s and converts it into a vec of vec<u16>s, with breaking at 0xfffe
/// then we can take each of those vec<u16> and convert it into a string
/// and the whole output can be a vec of strings. this is like the regex matches!  Yeet!
///
///

/// Take a sequence of u8 and make them into a bunch of strings yeeet
pub fn decode(input: Vec<u8>) -> Vec<Vec<String>> {
    // nesting [the three sequences [the strings in each [the char points in each]]]
    let mut output: Vec<Vec<Vec<u8>>> = vec![vec![vec![]]];
    let mut i: usize = 0;
    let mut str_num = 0;
    let mut seq_num = 0;
    while i < input.len() {
        let end_of_seq = i > 0 && input[i.clone() - 1] == 0xFF && input[i.clone()] == 0xFF;
        let end_of_str = i > 0 && input[i.clone() - 1] == 0xFF && input[i.clone()] == 0xFE;
        if end_of_seq {
            // begin a new sequence of strings

            // if previous code point wasn't a 0xffff, it must have beena 0xfffe
            // which means empty string
            output[seq_num.clone()].pop();
            seq_num += 1;
            str_num = 0;
            if i < input.len() {
                output.push(vec![vec![]]);
            }

            // if the next 2 bytes are FF FE, then we're at FF FE FF FF | FF FE
            // which means next byte we'll see an erronious FF FF sequence.
            if i.clone() + 1 < input.len() && input[i.clone() + 1] == 0xFF {
                //output[seq_num.clone()].push(vec![0xFF]); // this will get popped next loop
                i += 1; // incrament again.
            }
        } else if end_of_str {
            // begin a new string
            output[seq_num.clone()][str_num.clone()].pop(); // remove the 0xff from this
            output[seq_num.clone()].push(Vec::new());
            str_num += 1;
        } else {
            // add on to current string
            output[seq_num.clone()][str_num.clone()].push(input[i.clone()]);
        }
        i += 1;
    }
    output.pop();
    output.iter().map(|u| decode_seq(&u)).collect::<Vec<_>>()
}

fn decode_seq(seq_vect: &Vec<Vec<u8>>) -> Vec<String> {
    let mut out = Vec::with_capacity(seq_vect.len());
    for str_vect in seq_vect {
        out.push(String::from_utf8(str_vect.to_vec()).expect("oh no"));
    }
    out
}

pub fn encode(v: Vec<&Vec<String>>) -> Vec<u8> {
    let mut out: Vec<u8> = vec![];
    for glob in v {
        if glob.len() == 0 {
            // ensure correctness
            out.push(0xff);
            out.push(0xfe);
        } else {
            for s in glob.clone() {
                for byte in s.into_bytes() {
                    out.push(byte);
                }
                out.push(0xff);
                out.push(0xfe);
            }
        }
        out.push(0xff);
        out.push(0xff);
    }
    out
}

fn test() {
    // 0xFFFF - end of vect
    // 0xFFFE - end of str; start next item
    // from https://www.cl.cam.ac.uk/~mgk25/ucs/examples/UTF-8-test.txt
    // h
    /*
    let apple = vec![
        &vec![String::from("foo 𝄞 bar"), String::from("test!\\lol")],
        &vec![],
        &vec![],
        &vec![String::from("ne礡rd")]
    ];
    let raw = encode(apple);
    println!("{:#?}", raw);


    let output = decode (raw);
    println!{"{:#?}", &output};
    */
    println!("lol");
}
