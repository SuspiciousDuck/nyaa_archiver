// @generated automatically by Diesel CLI.

diesel::table! {
    comments (id) {
        id -> Integer,
        torrent_id -> Integer,
        submitter -> Text,
        date_created -> Integer,
        date_edited -> Nullable<Integer>,
        text -> Text,
    }
}

diesel::table! {
    deleted_torrents (id) {
        id -> Integer,
    }
}

diesel::table! {
    torrents (id) {
        id -> Integer,
        info_hash -> Nullable<Text>,
        seeders -> Integer,
        leechers -> Integer,
        completed -> Integer,
        title -> Text,
        category -> Integer,
        submitter -> Nullable<Text>,
        information -> Nullable<Text>,
        size -> BigInt,
        date -> Integer,
        description -> Nullable<Text>,
        comments -> Integer,
        remake -> Bool,
        trusted -> Bool,
        partial -> Bool,
        anonymous -> Bool,
        deleted -> Bool,
        last_updated -> Nullable<Integer>,
    }
}

diesel::table! {
    users (username) {
        username -> Text,
        password -> Nullable<Text>,
        salt -> Nullable<Text>,
        email -> Nullable<Text>,
        nyaa -> Bool,
        trusted -> Bool,
        banned -> Bool,
        last_updated -> Nullable<Integer>,
    }
}

diesel::joinable!(comments -> torrents (torrent_id));

diesel::allow_tables_to_appear_in_same_query!(
    comments,
    deleted_torrents,
    torrents,
    users,
);
