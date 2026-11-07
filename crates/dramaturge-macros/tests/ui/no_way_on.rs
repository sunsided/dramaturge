use dramaturge_macros::dialogue;

fn main() {
    let _ = dialogue! {
        with mara start intro {
            intro: "Hello." { -> names; }
            names: "They call me Mara." {
                set met_mara = true;
            }
        }
    };
}
