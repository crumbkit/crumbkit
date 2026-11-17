mod expansion;
mod refusals;

use proc_macro2::TokenStream;

use super::expand;

/// Returns the derive's output for `input` as text.
fn expanded(input: TokenStream) -> String {
    let input = syn::parse2(input).expect("the test input is a Rust type");
    expand(&input)
        .expect("the derive accepts the test input")
        .to_string()
}

/// Returns the message of the compile error that the derive gives for
/// `input`.
fn refusal(input: TokenStream) -> String {
    let input = syn::parse2(input).expect("the test input is a Rust type");
    match expand(&input) {
        Ok(output) => panic!("the derive accepted the input and wrote {output}"),
        Err(error) => error.to_string(),
    }
}
