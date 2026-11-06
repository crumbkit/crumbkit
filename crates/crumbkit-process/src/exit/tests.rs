use super::Status;

#[test]
fn the_shell_code_of_an_exit_code_is_that_code() {
    assert_eq!(Status::Code(0).shell_code(), 0);
    assert_eq!(Status::Code(7).shell_code(), 7);
}

#[test]
fn the_shell_code_of_a_signal_is_128_plus_its_number() {
    assert_eq!(Status::Signal(2).shell_code(), 130);
    assert_eq!(Status::Signal(9).shell_code(), 137);
}
