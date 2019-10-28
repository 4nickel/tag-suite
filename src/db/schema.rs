table! {
    file_tags (file_id, tag_id) {
        file_id -> BigInt,
        tag_id -> BigInt,
    }
}

table! {
    files (id) {
        id -> BigInt,
        kind -> BigInt,
        path -> Text,
    }
}

table! {
    tags (id) {
        id -> BigInt,
        name -> Text,
    }
}

joinable!(file_tags -> files (file_id));
joinable!(file_tags -> tags (tag_id));

allow_tables_to_appear_in_same_query!(
    file_tags,
    files,
    tags,
);
