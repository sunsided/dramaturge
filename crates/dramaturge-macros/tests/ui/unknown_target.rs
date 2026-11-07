use dramaturge_macros::dialogue;

fn main() {
    let _ = dialogue! {
        with mara start intro {
            intro: "Hello." {
                "Who are you?" => nmaes;
                "Goodbye." => end;
            }
            names: "Mara." { -> end; }
        }
    };
}
