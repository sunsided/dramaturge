use dramaturge_macros::dialogue;

fn main() {
    let _ = dialogue! {
        with mara start [again, intro] {
            intro: "Hello." { -> end; }
        }
    };
}
