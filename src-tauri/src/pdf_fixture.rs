//! Fictional ASCII PDF fixtures only. Not compiled into the application.
use lopdf::{dictionary, Document, Object, Stream};
pub fn document(pages: &[&[&str]]) -> Document {
    let mut doc = Document::with_version("1.4");
    let pages_id = doc.new_object_id();
    let font = doc.add_object(dictionary!{"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica","Encoding"=>"WinAnsiEncoding"});
    let mut kids = Vec::new();
    for lines in pages {
        let mut text = String::from("BT /F1 12 Tf 20 TL 72 720 Td\n");
        for line in *lines {
            let line = line
                .replace('\\', "\\\\")
                .replace('(', "\\(")
                .replace(')', "\\)");
            text.push_str(&format!("({line}) Tj T*\n"));
        }
        text.push_str("ET\n");
        let stream = doc.add_object(Stream::new(dictionary! {}, text.into_bytes()));
        let page = doc.add_object(dictionary!{"Type"=>"Page","Parent"=>pages_id,"Contents"=>stream,
            "MediaBox"=>vec![0.into(),0.into(),612.into(),792.into()],"Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font}}});
        kids.push(Object::Reference(page));
    }
    doc.objects.insert(
        pages_id,
        dictionary! {"Type"=>"Pages","Kids"=>kids,"Count"=>pages.len() as i64}.into(),
    );
    let catalog = doc.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages_id});
    doc.trailer.set("Root", catalog);
    doc.trailer.set(
        "ID",
        vec![
            Object::string_literal("synthetic-id-one"),
            Object::string_literal("synthetic-id-two"),
        ],
    );
    doc
}
pub fn bytes(mut doc: Document) -> Vec<u8> {
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}
pub fn statement() -> Vec<u8> {
    bytes(document(&[
        &[
            "Synthetic checking statement",
            "2026-10-01 Fictional payroll 1000.01",
            "2026-10-02 Fictional groceries -25.11",
        ],
        &[
            "2026-10-03 Pending merchant",
            "2026-10-04 Two numeric columns -10.00 900.00",
        ],
    ]))
}
pub fn protected() -> Vec<u8> {
    protected_with_password("Fictional PDF phrase")
}
pub fn protected_with_password(user_password: &str) -> Vec<u8> {
    let mut doc = document(&[&["2026-10-01 Protected synthetic pay 100"]]);
    let state = lopdf::EncryptionState::try_from(lopdf::EncryptionVersion::V2 {
        document: &doc,
        owner_password: "Fictional owner phrase",
        user_password,
        key_length: 128,
        permissions: lopdf::Permissions::all(),
    })
    .unwrap();
    doc.encrypt(&state).unwrap();
    bytes(doc)
}
