// @generated automatically by Diesel CLI.

diesel::table! {
    comments (id) {
        id -> Integer,
        torrent_id -> Integer,
        submitter -> Text,
        date_created -> BigInt,
        date_edited -> Nullable<BigInt>,
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
        info_hash -> Text,
        seeders -> Integer,
        leechers -> Integer,
        completed -> Integer,
        title -> Text,
        category -> Integer,
        submitter -> Nullable<Text>,
        information -> Nullable<Text>,
        size -> BigInt,
        date -> BigInt,
        description -> Nullable<Text>,
        comments -> Integer,
        remake -> Bool,
        trusted -> Bool,
        partial -> Bool,
        anonymous -> Bool,
        deleted -> Bool,
        hidden -> Bool,
        next_update -> Nullable<BigInt>,
        update_count -> Integer,
        update_frequency -> Nullable<Integer>,
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
        nyaa_admin -> Bool,
        nyaa_mod -> Bool,
        avatar -> Nullable<Text>,
    }
}

diesel::joinable!(comments -> torrents (torrent_id));

diesel::allow_tables_to_appear_in_same_query!(comments, deleted_torrents, torrents, users,);
