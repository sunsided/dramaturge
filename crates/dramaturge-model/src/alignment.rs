use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AlignmentAxisId(pub Arc<str>);

impl AlignmentAxisId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignmentAxis {
    pub id: AlignmentAxisId,
    pub display_name: Arc<str>,
    pub poles: Vec<AlignmentPole>,
}

impl AlignmentAxis {
    pub fn new(id: impl Into<Arc<str>>, display_name: impl Into<Arc<str>>) -> Self {
        Self {
            id: AlignmentAxisId::new(id),
            display_name: display_name.into(),
            poles: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AlignmentPoleId(pub Arc<str>);

impl AlignmentPoleId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignmentPole {
    pub id: AlignmentPoleId,
    pub display_name: Arc<str>,
}

impl AlignmentPole {
    pub fn new(id: impl Into<Arc<str>>, display_name: impl Into<Arc<str>>) -> Self {
        Self {
            id: AlignmentPoleId::new(id),
            display_name: display_name.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignmentScore {
    pub axis: AlignmentAxisId,
    pub pole: AlignmentPoleId,
    pub value: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alignment_axis_construction() {
        let axis = AlignmentAxis::new("moral", "Moral Alignment");
        assert_eq!(&*axis.id.0, "moral");
        assert_eq!(&*axis.display_name, "Moral Alignment");
        assert!(axis.poles.is_empty());
    }

    #[test]
    fn alignment_pole_construction() {
        let pole = AlignmentPole::new("paragon", "Paragon");
        assert_eq!(&*pole.id.0, "paragon");
        assert_eq!(&*pole.display_name, "Paragon");
    }

    #[test]
    fn alignment_score_construction() {
        let score = AlignmentScore {
            axis: AlignmentAxisId::new("moral"),
            pole: AlignmentPoleId::new("paragon"),
            value: 50,
        };
        assert_eq!(score.value, 50);
    }
}
