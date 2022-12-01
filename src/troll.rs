use rand::Rng;

lazy_static! {
    static ref AYAKA_QUOTES: Vec<&'static str> = vec![
        "I only wish life could be as leisurely as this a little more often... How greedy of me.",
        "A blade is like a tea-leaf. Only those who sample it many times can appreciate its true qualities.",
        "So this is a day in the life of the Traveler... Hehe, I'm learning more about you all the time.",
        "Master of Inazuma Kamisato Art Tachi Jutsu — Kamisato Ayaka, present! Delighted to make your acquaintance.",
        "Come with me, let us find shelter from the rain.",
        "There's a pleasant breeze and glorious sunshine... So, where shall we go for a walk?",
        "Close your eyes and face the wind... It feels wonderful, doesn't it?",
        "Oh, good morning, Traveler. ...Whenever I see you in the morning, somehow, it makes me feel like... today is going to be a good day.",
        "Good afternoon. It is normal to feel drowsy after lunch, might I interest you in a game of Go to stimulate the mind?",
        "Greetings. An auspicious breeze blows this evening. Tonight will be peaceful.",
        "\"Was it one's thoughts that drew him to my dreams? Had I known it a dream, one would not have awakened.\" Hehe, I love that poem.",
        "Ah, little Sayu. She hasn't been causing you any trouble lately, has she? Hehe, if you ever notice her slacking off, please let me know.",
        "Thank you very much. It has been richly rewarding to learn from you so far, and I believe my skills with the blade can go even further still.",
        "Thank you for your guidance. With your assistance, I am gaining a more thorough understanding of my capabilities.",
        "Oh, good morning, Traveler. ...Whenever I see you in the morning, somehow, it makes me feel like... today is going to be a good day.",
        "Oh, good morning, Traveler. ...Whenever I see you in the morning, somehow, it makes me feel like... today is going to be a good day.",
        "Oh, good morning, Traveler. ...Whenever I see you in the morning, somehow, it makes me feel like... today is going to be a good day.",
        "Oh, good morning, Traveler. ...Whenever I see you in the morning, somehow, it makes me feel like... today is going to be a good day.",
    ];
}

pub fn random_ayaka_quote() -> &'static str {
    AYAKA_QUOTES[rand::thread_rng().gen_range(0..AYAKA_QUOTES.len())]
}