//! Demonstrates the tag system for dramaturge dialogue options.
//!
//! This example shows:
//! 1. Creating tags via `CustomDialogTag::new("axis", "key")`
//! 2. Implementing `DialogTag` on a custom domain-specific enum
//! 3. Using tags in `DialogueOption` structures

use dramaturge_model::{
    CharacterId, CustomDialogTag, DialogTag, DialogTagKind, DialogTarget, DialogText,
    DialogueGraph, DialogueNode, DialogueNodeId, DialogueOption, EntityId,
};

/// Domain-specific tag for a fantasy RPG.
///
/// Demonstrates how to implement the `DialogTag` trait on your own enum
/// to provide typed, IDE-friendly tags while still conforming to the
/// `DialogTag` contract.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
enum FantasyTag {
    /// Moral alignment: good vs evil
    Alignment(Alignment),
    /// Required skill to succeed
    SkillCheck(Skill),
    /// Type of speech act
    Speech(SpeechType),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
enum Alignment {
    Good,
    Neutral,
    Evil,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
enum Skill {
    Persuasion,
    Intimidation,
    Lore,
    Stealth,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
enum SpeechType {
    Friendly,
    Hostile,
    Flirt,
    Joke,
}

impl DialogTag for FantasyTag {
    fn axis(&self) -> &'static str {
        match self {
            FantasyTag::Alignment(_) => "alignment",
            FantasyTag::SkillCheck(_) => "skill",
            FantasyTag::Speech(_) => "speech",
        }
    }

    fn key(&self) -> &'static str {
        match self {
            FantasyTag::Alignment(a) => match a {
                Alignment::Good => "good",
                Alignment::Neutral => "neutral",
                Alignment::Evil => "evil",
            },
            FantasyTag::SkillCheck(s) => match s {
                Skill::Persuasion => "persuasion",
                Skill::Intimidation => "intimidation",
                Skill::Lore => "lore",
                Skill::Stealth => "stealth",
            },
            FantasyTag::Speech(s) => match s {
                SpeechType::Friendly => "friendly",
                SpeechType::Hostile => "hostile",
                SpeechType::Flirt => "flirt",
                SpeechType::Joke => "joke",
            },
        }
    }
}

fn main() {
    // --- 1. CustomDialogTag usage ---
    println!("=== CustomDialogTag ===");

    let tag1 = CustomDialogTag::new("faction", "rebels");
    let tag2 = CustomDialogTag::new("moral", "paragon");
    let tag3 = CustomDialogTag::new("skill", "persuasion");

    println!("tag1: axis={}, key={}", tag1.axis, tag1.key);
    println!("tag2: axis={}, key={}", tag2.axis, tag2.key);
    println!("tag3: axis={}, key={}", tag3.axis, tag3.key);

    assert_eq!(tag1, CustomDialogTag::new("faction", "rebels"));
    println!("Equality check passed!\n");

    // --- 2. DialogTagKind (type alias for CustomDialogTag) ---
    println!("=== DialogTagKind ===");

    let kind: DialogTagKind = DialogTagKind::new("tone", "serious");
    println!("kind: axis={}, key={}", kind.axis, kind.key);
    println!("DialogTagKind is a type alias for CustomDialogTag\n");

    // --- 3. Custom DialogTag implementation ---
    println!("=== Custom DialogTag impl ===");

    let fantasy_tags = vec![
        FantasyTag::Alignment(Alignment::Good),
        FantasyTag::SkillCheck(Skill::Persuasion),
        FantasyTag::Speech(SpeechType::Flirt),
    ];

    for tag in &fantasy_tags {
        println!("FantasyTag: axis={}, key={}", tag.axis(), tag.key());
    }
    println!();

    // --- 4. Tags in a DialogueGraph ---
    println!("=== Tags in DialogueGraph ===");

    let mut graph = DialogueGraph::new();

    let speaker = CharacterId::new(EntityId::new(1));
    let node = DialogueNode::new(1, speaker, DialogText::inline("Greetings, traveler."));

    let option1 = DialogueOption::new(
        1,
        DialogText::inline("I come in peace."),
        DialogTarget::node(2),
        DialogueNodeId::new(1),
    );
    let option2 = DialogueOption::new(
        2,
        DialogText::inline("Hand over your gold!"),
        DialogTarget::node(2),
        DialogueNodeId::new(1),
    );
    let option3 = DialogueOption::new(
        3,
        DialogText::inline("Tell me about the old kingdom."),
        DialogTarget::end(),
        DialogueNodeId::new(1),
    );

    // Attach CustomDialogTag to options
    let mut option1 = option1;
    option1.tags.push(CustomDialogTag::new("tone", "peaceful"));
    option1
        .tags
        .push(CustomDialogTag::new("speech", "friendly"));

    let mut option2 = option2;
    option2.tags.push(CustomDialogTag::new("tone", "hostile"));
    option2.tags.push(CustomDialogTag::new("speech", "threat"));

    let mut option3 = option3;
    option3.tags.push(CustomDialogTag::new("speech", "ask"));
    option3.tags.push(CustomDialogTag::new("topic", "lore"));

    let mut node = node;
    node.options.push(option1);
    node.options.push(option2);
    node.options.push(option3);

    graph.nodes.push(node);

    // Verify tags are attached
    let node = &graph.nodes[0];
    println!(
        "Node: {}",
        match &node.text {
            DialogText::Inline(s) => s.as_str(),
            _ => "<localized>",
        }
    );

    for opt in &node.options {
        let text = match &opt.text {
            DialogText::Inline(s) => s.as_str(),
            _ => "<localized>",
        };
        let tag_str: Vec<String> = opt
            .tags
            .iter()
            .map(|t| format!("{}:{}", t.axis, t.key))
            .collect();
        println!("  Option: '{}' -> tags=[{}]", text, tag_str.join(", "));
    }

    println!("\nExample completed successfully!");
}
