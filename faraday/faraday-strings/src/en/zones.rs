//! The fixed table of time zones the Set the time page chooses from: one
//! row per offset, with city names as the clue. A fixed table, no zone
//! database and no daylight saving rules: the person changes the row twice
//! a year where their place does.

/// Each row is the offset in minutes east of UTC and the cities clue,
/// ordered west to east. A city that is on that offset only part of the
/// year says "in winter" or "in summer" as the people there call the season.
pub const ZONES: &[(i16, &str)] = &[
    (-720, "Baker Island, Howland Island"),
    (-660, "Pago Pago, Niue"),
    (-600, "Honolulu, Papeete"),
    (-570, "Marquesas Islands"),
    (-540, "Anchorage in winter, Gambier Islands"),
    (
        -480,
        "Los Angeles in winter, Vancouver in winter, Anchorage in summer",
    ),
    (-420, "Denver in winter, Phoenix, Los Angeles in summer"),
    (-360, "Chicago in winter, Mexico City, Denver in summer"),
    (-300, "New York in winter, Chicago in summer, Bogota, Lima"),
    (
        -240,
        "Halifax in winter, New York in summer, Caracas, Santiago in winter",
    ),
    (-210, "St. John's in winter"),
    (
        -180,
        "Sao Paulo, Buenos Aires, Halifax in summer, Santiago in summer",
    ),
    (-120, "Nuuk in winter, South Georgia, Fernando de Noronha"),
    (-60, "Azores in winter, Cape Verde, Nuuk in summer"),
    (0, "UTC · Reykjavik, London in winter"),
    (
        60,
        "Paris in winter, Berlin in winter, London in summer, Lagos",
    ),
    (
        120,
        "Johannesburg, Athens in winter, Cairo in winter, Berlin in summer",
    ),
    (180, "Moscow, Istanbul, Riyadh, Nairobi"),
    (210, "Tehran"),
    (240, "Dubai, Baku"),
    (270, "Kabul"),
    (300, "Karachi, Tashkent, Almaty"),
    (330, "Delhi, Mumbai, Colombo"),
    (345, "Kathmandu"),
    (360, "Dhaka, Omsk, Bishkek"),
    (390, "Yangon, Cocos Islands"),
    (420, "Bangkok, Jakarta, Ho Chi Minh City"),
    (480, "Singapore, Beijing, Hong Kong, Perth"),
    (525, "Eucla"),
    (540, "Tokyo, Seoul"),
    (570, "Darwin, Adelaide in winter"),
    (
        600,
        "Brisbane, Sydney in winter, Melbourne in winter, Vladivostok",
    ),
    (630, "Adelaide in summer, Lord Howe Island in winter"),
    (
        660,
        "Sydney in summer, Melbourne in summer, Noumea, Honiara",
    ),
    (720, "Auckland in winter, Fiji, Petropavlovsk-Kamchatsky"),
    (765, "Chatham Islands in winter"),
    (780, "Auckland in summer, Nuku'alofa, Apia"),
    (840, "Kiritimati"),
];
