use super::*;

#[test]
fn new_id_is_only_id_and_newline_without_notes() {
    let data = NewIdDto {
        id: "01M4CC0VMSZ7BRS740C8VSQ5ZK".into(),
    };
    let output = new_id(&data);
    assert_eq!(output.stdout, "01M4CC0VMSZ7BRS740C8VSQ5ZK\n");
    assert!(output.notes.is_empty());
}
