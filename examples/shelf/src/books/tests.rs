//! Tests for the book catalogue and its rendering.

use super::{CATALOGUE, write_list};

#[test]
fn the_list_is_a_header_and_one_aligned_row_per_book() {
    let mut out = Vec::new();

    write_list(&CATALOGUE, &mut out).unwrap();

    let expected = "\
ID  TITLE                              AUTHOR              YEAR
1   Pride and Prejudice                Jane Austen         1813
2   Frankenstein                       Mary Shelley        1818
3   Moby-Dick                          Herman Melville     1851
4   Great Expectations                 Charles Dickens     1861
5   Middlemarch                        George Eliot        1871
6   The Adventures of Sherlock Holmes  Arthur Conan Doyle  1892
";
    assert_eq!(String::from_utf8(out).unwrap(), expected);
}
