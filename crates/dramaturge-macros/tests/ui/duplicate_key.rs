use dramaturge_macros::dialogue;

fn main() {
    let _ = dialogue! {
        with mara start intro {
            intro: "Hello." { -> end; }
            intro: "Hello again." { -> end; }
        }
    };
}
