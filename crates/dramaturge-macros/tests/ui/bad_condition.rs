use dramaturge_macros::dialogue;

fn main() {
    let _ = dialogue! {
        with mara start intro {
            intro: "Hello." {
                "Pay." when gold >= lots => end;
                "Goodbye." => end;
            }
        }
    };
}
