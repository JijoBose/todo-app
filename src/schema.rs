// @generated automatically by Diesel CLI.

diesel::table! {
    tasks (id) {
        id -> Text,
        name -> Text,
        done -> Bool,
    }
}
