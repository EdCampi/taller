//! Tests unitarios del módulo commands.

#[cfg(test)]
mod command_tests {
    // IMPORTS
    use crate::command::commands::CommandError;
    use crate::command::types::Command;
    use crate::command::*;
    use crate::storage::DataStore;
    use std::collections::HashSet;

    // CONSTANTES
    static ERR_WRONG_NUM_ARGS: &str = "ERR wrong number of arguments for '_' command";

    // FUNCIONES AUXILIARES

    /// Crea un `DataStore`, agregando en en `list_db`,
    /// `"DPS" = ["Ashe", "F.R.E.D", "B.O.B", "Torbjorn", "Echo"]`
    fn set_up_data_store_with_multiple_items_list() -> DataStore {
        let mut store = DataStore::new();
        store.list_db.insert(
            "DPS".to_string(),
            vec![
                "Ashe".to_string(),
                "F.R.E.D".to_string(),
                "B.O.B".to_string(),
                "Torbjorn".to_string(),
                "Echo".to_string(),
            ],
        );
        store
    }

    /// Crea un `DataStore`, agregando en `set_db`,
    /// `"Maps" = {"El Dorado", "Petra", "Busan"}`
    fn set_up_data_store_with_multiple_items_set() -> DataStore {
        let mut store = DataStore::new();
        let mut set = HashSet::new();
        set.insert("El Dorado".to_string());
        set.insert("Petra".to_string());
        set.insert("Busan".to_string());
        store.set_db.insert("Maps".to_string(), set);
        store
    }

    // TESTS

    /* STRING TESTS */

    /* APPEND */

    #[test]
    fn append_creates_a_new_value_on_non_existent_key() {
        let mut store = DataStore::new();
        let cmd = Command::Append("Moira".to_string(), "DPS".to_string());
        let result = cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::Int(3));
        assert_eq!(store.string_db.get("Moira").unwrap(), "DPS");
    }

    #[test]
    fn append_adds_its_value_to_an_existing_key() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Siblings".to_string(), "Hanzo".to_string());

        let cmd = Command::Append("Siblings".to_string(), "-Genji".to_string());
        let result = cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::Int(11));
        assert_eq!(store.string_db.get("Siblings").unwrap(), "Hanzo-Genji");
    }

    #[test]
    fn append_doesnt_work_for_a_set() {
        let mut store = set_up_data_store_with_multiple_items_set();

        let cmd = Command::Append("Maps".to_string(), "Redwood dam".to_string());
        let result = cmd.execute_write(&mut store);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn append_doesnt_work_for_a_list() {
        let mut store = set_up_data_store_with_multiple_items_list();

        let cmd = Command::Append("DPS".to_string(), "McCree".to_string());
        let result = cmd.execute_write(&mut store);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    /* DEL */

    #[test]
    fn del_works_for_existing_keys() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Latino".to_string(), "Illari".to_string());
        store.list_db.insert(
            "Asian".to_string(),
            vec!["Kiriko".to_string(), "Hanzo".to_string()],
        );
        store
            .set_db
            .insert("European".to_string(), HashSet::from(["Zarya".to_string()]));

        let del_cmd = Command::Del(vec!["Latino".to_string(), "Asian".to_string()]);
        let result = del_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::Int(2));
        assert!(store.string_db.get("Latino").is_none());
        assert!(store.list_db.get("Asian").is_none());
        assert!(store.set_db.get("European").is_some());
    }

    #[test]
    fn del_works_for_nonexistent_key() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Map".to_string(), "Petra".to_string());

        let del_cmd = Command::Del(vec!["DPS".to_string()]);
        let result = del_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::Int(0));
        assert!(store.string_db.get("Map").is_some());
    }

    #[test]
    fn del_works_for_nonexistent_keys() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Map".to_string(), "Petra".to_string());

        let del_cmd = Command::Del(vec![
            "TANK".to_string(),
            "DPS".to_string(),
            "SUP".to_string(),
        ]);
        let result = del_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::Int(0));
        assert!(store.string_db.get("Map").is_some());
    }

    #[test]
    fn del_works_for_mixed_existing_and_nonexistent_keys() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Map1".to_string(), "Petra".to_string());
        store
            .list_db
            .insert("Map2".to_string(), vec!["Busan".to_string()]);

        let del_cmd = Command::Del(vec!["Map1".to_string(), "Map3".to_string()]);
        let result = del_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::Int(1));
        assert!(store.string_db.get("Map1").is_none());
        assert!(store.list_db.get("Map2").is_some());
    }

    #[test]
    fn del_doenst_works_for_empty_keys() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Map1".to_string(), "Busan".to_string());
        store
            .list_db
            .insert("Map2".to_string(), vec!["Busan".to_string()]);

        let del_cmd = Command::Del(vec![]);
        let result = del_cmd.execute_write(&mut store);

        let _exp_err = ERR_WRONG_NUM_ARGS.replace("_", "del");
        assert!(matches!(result.unwrap_err(), CommandError::WrongNumArgs));
        assert!(store.string_db.get("Map1").is_some());
        assert!(store.list_db.get("Map2").is_some());
    }

    /* ECHO */

    #[test]
    fn echo_works_for_empty_string() {
        let mut empty_store = DataStore::new();
        let cmd = Command::Echo("".to_string());

        let result = cmd.execute_read(&mut empty_store, None, None, None, None, None);
        assert_eq!(result.unwrap(), ResponseType::Str("".to_string()));
    }

    #[test]
    fn echo_works_for_an_string() {
        // Configuración inicial
        let mut empty_store = DataStore::new();
        let argument = "Hello World".to_string();
        let cmd = Command::Echo(argument.clone());

        // Ejecutar el comando
        let result = cmd.execute_read(&mut empty_store, None, None, None, None, None);

        // Verificar el resultado
        assert_eq!(result.unwrap(), ResponseType::Str(argument));
    }

    #[test]
    fn echo_works_for_non_empty_string() {
        let mut empty_store = DataStore::new();
        let cmd = Command::Echo("I need healing".to_string());

        let result = cmd.execute_read(&mut empty_store, None, None, None, None, None);
        assert_eq!(
            result.unwrap(),
            ResponseType::Str("I need healing".to_string())
        );
    }

    #[test]
    fn echo_doesnt_ignore_special_characters() {
        let mut empty_store = DataStore::new();
        let cmd = Command::Echo("Clash\nPush".to_string());

        let result = cmd.execute_read(&mut empty_store, None, None, None, None, None);
        assert_eq!(
            result.unwrap(),
            ResponseType::Str("Clash\nPush".to_string())
        );
    }

    /* GET */

    #[test]
    fn get_works() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("DPS_2".to_string(), "Moira".to_string());

        let get_cmd = Command::Get("DPS_2".to_string());
        let result = get_cmd.execute_read(&mut store, None, None, None, None, None);
        assert_eq!(result.unwrap(), ResponseType::Str("Moira".to_string()));
        assert_eq!(store.string_db.get("DPS_2").unwrap(), "Moira");
    }

    #[test]
    fn get_works_over_non_existent_key() {
        let mut store = DataStore::new();
        let get_cmd = Command::Get("DPS".to_string());
        let result = get_cmd.execute_read(&mut store, None, None, None, None, None);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ResponseType::Null(None));
    }

    #[test]
    fn get_doesnt_work_over_list() {
        let mut store = set_up_data_store_with_multiple_items_list();
        let get_cmd = Command::Get("DPS".to_string());
        let result = get_cmd.execute_read(&mut store, None, None, None, None, None);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
        let list = store.list_db.get("DPS").unwrap();
        assert_eq!(list.len(), 5);
        assert_eq!(list[0], "Ashe".to_string());
        assert_eq!(list[1], "F.R.E.D".to_string());
        assert_eq!(list[2], "B.O.B".to_string());
        assert_eq!(list[3], "Torbjorn".to_string());
        assert_eq!(list[4], "Echo".to_string());
    }

    #[test]
    fn get_doesnt_work_over_set() {
        let mut store = set_up_data_store_with_multiple_items_set();
        let get_cmd = Command::Get("Maps".to_string());
        let result = get_cmd.execute_read(&mut store, None, None, None, None, None);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
        let expected: HashSet<String> = ["El Dorado", "Petra", "Busan"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let set_val = store.set_db.get("Maps").unwrap();
        assert_eq!(set_val, &expected);
    }

    /* GETDEL */

    #[test]
    fn getdel_works_for_existing_string() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Ashe".to_string(), "B.O.B".to_string());

        let getdel_cmd = Command::Getdel("Ashe".to_string());
        let result = getdel_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::Str("B.O.B".to_string()));
        assert!(store.get("Ashe").is_none());
    }

    #[test]
    fn getdel_doesnt_work_for_existing_list() {
        let mut store = DataStore::new();
        store.list_db.insert(
            "Ashe".to_string(),
            vec!["B.O.B".to_string(), "F.R.E.D".to_string()],
        );

        let getdel_cmd = Command::Getdel("Ashe".to_string());
        let result = getdel_cmd.execute_write(&mut store);

        if let Some(list) = store.list_db.get("Ashe") {
            assert_eq!(list.len(), 2);
            assert_eq!(list[0], "B.O.B".to_string());
            assert_eq!(list[1], "F.R.E.D".to_string());
        }
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn getdel_doesnt_works_for_existing_set() {
        let mut store = DataStore::new();
        let mut set = HashSet::new();
        set.insert("Genji".to_string());
        set.insert("Reaper".to_string());
        store.set_db.insert("DPS".to_string(), set.clone());

        let getdel_cmd = Command::Getdel("DPS".to_string());
        let result = getdel_cmd.execute_write(&mut store);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
        assert!(store.set_db.get("DPS").is_some());
    }

    #[test]
    fn getdel_returns_empty_for_nonexistent_key() {
        let mut empty_store = DataStore::new();

        let getdel_cmd = Command::Getdel("NonExistent".to_string());
        let result = getdel_cmd.execute_write(&mut empty_store);

        assert_eq!(result.unwrap(), ResponseType::Null(None));
        assert!(empty_store.string_db.get("NonExistent").is_none());
        assert!(empty_store.list_db.get("NonExistent").is_none());
        assert!(empty_store.set_db.get("NonExistent").is_none());
    }

    /* GETRANGE */

    #[test]
    fn getrange_works_for_an_string() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Llave1".to_string(), "Liverpool".to_string());
        let getrange_cmd = Command::Getrange("Llave1".to_string(), 1, 20);
        let string_expected = "iverpool".to_string();

        let result = getrange_cmd.execute_read(&mut store, None, None, None, None, None);
        assert_eq!(result.unwrap(), ResponseType::Str(string_expected));
    }

    #[test]
    fn getrange_works_for_existing_string() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Ashe".to_string(), "B.O.B".to_string());

        let getrange_cmd = Command::Getrange("Ashe".to_string(), 0, 2);
        let result = getrange_cmd.execute_read(&mut store, None, None, None, None, None);
        let exp_value = "B.O".to_string();
        assert_eq!(result.unwrap(), ResponseType::Str(exp_value));
    }

    #[test]
    fn getrange_works_for_existing_string_with_negative_start() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Ashe".to_string(), "B.O.B".to_string());

        let getrange_cmd = Command::Getrange("Ashe".to_string(), -3, -1);
        let result = getrange_cmd.execute_read(&mut store, None, None, None, None, None);
        let exp_value = "O.B".to_string();
        assert_eq!(result.unwrap(), ResponseType::Str(exp_value));
    }

    #[test]
    fn getrange_works_for_existing_string_with_negative_end() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Ashe".to_string(), "B.O.B".to_string());

        let getrange_cmd = Command::Getrange("Ashe".to_string(), 0, -2);
        let result = getrange_cmd.execute_read(&mut store, None, None, None, None, None);
        let exp_value = "B.O.".to_string();
        assert_eq!(result.unwrap(), ResponseType::Str(exp_value));
    }

    #[test]
    fn getrange_works_for_non_existing_string() {
        let mut empty_store = DataStore::new();
        let getrange_cmd = Command::Getrange("NonExistent".to_string(), 0, 100);
        let result = getrange_cmd.execute_read(&mut empty_store, None, None, None, None, None);
        let exp_value = "".to_string();
        assert_eq!(result.unwrap(), ResponseType::Str(exp_value));
    }

    #[test]
    fn getrange_doesnt_work_for_existing_list() {
        let mut store = DataStore::new();
        store.list_db.insert(
            "Ashe".to_string(),
            vec!["B.O.B".to_string(), "F.R.E.D".to_string()],
        );
        let getrange_cmd = Command::Getrange("Ashe".to_string(), 0, 2);
        let result = getrange_cmd.execute_read(&mut store, None, None, None, None, None);
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn getrange_doesnt_work_for_existing_set() {
        let mut store = DataStore::new();
        let mut set = HashSet::new();
        set.insert("Genji".to_string());
        set.insert("Reaper".to_string());
        store.set_db.insert("DPS".to_string(), set);

        let getrange_cmd = Command::Getrange("DPS".to_string(), 0, 2);
        let result = getrange_cmd.execute_read(&mut store, None, None, None, None, None);
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    /* SET */

    #[test]
    fn set_works() {
        let mut store = DataStore::new();
        let set_cmd = Command::Set("DPS_1".to_string(), "Junkrat".to_string());
        let result = set_cmd.execute_write(&mut store);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ResponseType::Str("OK".to_string()));
        assert_eq!(store.string_db.get("DPS_1").unwrap(), "Junkrat");
    }

    #[test]
    fn set_works_over_list() {
        let mut store = DataStore::new();
        store
            .list_db
            .insert("DPS".to_string(), vec!["Reaper".to_string()]);

        let set_cmd = Command::Set("DPS".to_string(), "Mei".to_string());
        let result = set_cmd.execute_write(&mut store);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ResponseType::Str("OK".to_string()));
        assert_eq!(store.string_db.get("DPS").unwrap(), "Mei");
        assert!(store.list_db.get("DPS").is_none());
    }

    #[test]
    fn set_works_over_set() {
        let mut store = DataStore::new();
        let mut set_aux = HashSet::new();
        set_aux.insert("Ana".to_string());
        set_aux.insert("Juno".to_string());
        store.set_db.insert("SUPS".to_string(), set_aux);

        let set_cmd = Command::Set("SUPS".to_string(), "Mercy".to_string());
        let result = set_cmd.execute_write(&mut store);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ResponseType::Str("OK".to_string()));
        assert_eq!(store.string_db.get("SUPS").unwrap(), "Mercy");
        assert!(store.set_db.get("SUPS").is_none());
    }

    /* STRLEN */

    #[test]
    fn strlen_works_for_an_empty_string() {
        let mut store = DataStore::new();
        store.string_db.insert("Empty".to_string(), "".to_string());

        let strlen_cmd = Command::Strlen("Empty".to_string());
        let result = strlen_cmd.execute_read(&mut store, None, None, None, None, None);
        let exp_value = 0;
        assert_eq!(result.unwrap(), ResponseType::Int(exp_value));
    }

    #[test]
    fn strlen_works_for_a_string() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Ashe".to_string(), "B.O.B".to_string());

        let strlen_cmd = Command::Strlen("Ashe".to_string());
        let result = strlen_cmd.execute_read(&mut store, None, None, None, None, None);
        let exp_value = 5;
        assert_eq!(result.unwrap(), ResponseType::Int(exp_value));
    }

    #[test]
    fn strlen_0_for_no_value() {
        let mut empty_store = DataStore::new();

        let strlen_cmd = Command::Strlen("No existe".to_string());
        let result = strlen_cmd.execute_read(&mut empty_store, None, None, None, None, None);
        let exp_value = 0;
        assert_eq!(result.unwrap(), ResponseType::Int(exp_value));
    }

    #[test]
    fn strlen_doesnt_work_for_a_list() {
        let mut store = DataStore::new();
        store
            .list_db
            .insert("Ashe".to_string(), vec!["B.O.B".to_string()]);

        let strlen_cmd = Command::Strlen("Ashe".to_string());
        let result = strlen_cmd.execute_read(&mut store, None, None, None, None, None);
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn strlen_doesnt_work_for_a_set() {
        let mut store = DataStore::new();
        let mut set = HashSet::new();
        set.insert("King's Row".to_string());
        store.set_db.insert("Maps".to_string(), set);

        let strlen_cmd = Command::Strlen("Maps".to_string());
        let result = strlen_cmd.execute_read(&mut store, None, None, None, None, None);
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    /* SUBSTR */

    #[test]
    fn substr_works_for_an_string() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Llave1".to_string(), "Somos todos Montiel".to_string());
        let substr_cmd = Command::Substr("Llave1".to_string(), 0, 4);
        let string_expected = "Somos".to_string();

        let result = substr_cmd.execute_read(&mut store, None, None, None, None, None);
        assert_eq!(result.unwrap(), ResponseType::Str(string_expected));
    }

    #[test]
    fn substr_doesnt_work_for_a_list() {
        let mut store = DataStore::new();
        store.list_db.insert(
            "Llave1".to_string(),
            vec!["Somos todos Montiel".to_string()],
        );
        let substr_cmd = Command::Substr("Llave1".to_string(), 0, 4);

        let result = substr_cmd.execute_read(&mut store, None, None, None, None, None);
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn substr_doesnt_work_for_a_set() {
        let mut store = DataStore::new();
        let mut set = HashSet::new();
        set.insert("King's Row".to_string());
        store.set_db.insert("Maps".to_string(), set);
        let substr_cmd = Command::Substr("Maps".to_string(), 0, 4);

        let result = substr_cmd.execute_read(&mut store, None, None, None, None, None);
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    /* LIST TESTS */

    /* LLEN */

    #[test]
    fn llen_works_for_an_empty_list() {
        let mut store = DataStore::new();
        store.list_db.insert("Empty".to_string(), vec![]);

        let llen_cmd = Command::Llen("Empty".to_string());
        let result = llen_cmd.execute_read(&mut store, None, None, None, None, None);
        let exp_value = 0;
        assert_eq!(result.unwrap(), ResponseType::Int(exp_value));
    }

    #[test]
    fn llen_works_for_a_list_with_one_items() {
        let mut store = DataStore::new();
        store
            .list_db
            .insert("Ashe".to_string(), vec!["B.O.B".to_string()]);

        let llen_cmd = Command::Llen("Ashe".to_string());
        let result = llen_cmd.execute_read(&mut store, None, None, None, None, None);
        let exp_value = 1;
        assert_eq!(result.unwrap(), ResponseType::Int(exp_value));
    }

    #[test]
    fn llen_works_for_a_list_with_multiple_items() {
        let mut store = DataStore::new();
        store.list_db.insert(
            "Ashe".to_string(),
            vec!["B.O.B".to_string(), "F.R.E.D".to_string()],
        );

        let llen_cmd = Command::Llen("Ashe".to_string());
        let result = llen_cmd.execute_read(&mut store, None, None, None, None, None);
        let exp_value = 2;
        assert_eq!(result.unwrap(), ResponseType::Int(exp_value));
    }

    #[test]
    fn llen_0_for_no_value() {
        let mut empty_store = DataStore::new();

        let llen_cmd = Command::Llen("No existe".to_string());
        let result = llen_cmd.execute_read(&mut empty_store, None, None, None, None, None);
        let exp_value = 0;
        assert_eq!(result.unwrap(), ResponseType::Int(exp_value));
    }

    #[test]
    fn llen_doesnt_work_for_a_string() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Ashe".to_string(), "B.O.B".to_string());

        let llen_cmd = Command::Llen("Ashe".to_string());
        let result = llen_cmd.execute_read(&mut store, None, None, None, None, None);
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn llen_doesnt_work_for_a_set() {
        let mut store = DataStore::new();
        let mut set = HashSet::new();
        set.insert("King's Row".to_string());
        store.set_db.insert("Maps".to_string(), set);

        let llen_cmd = Command::Llen("Maps".to_string());
        let result = llen_cmd.execute_read(&mut store, None, None, None, None, None);
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    /* LPOP */

    #[test]
    fn lpop_empty_list() {
        let mut store = DataStore::new();
        store.list_db.insert("EmptyList".to_string(), vec![]);

        let lpop_cmd = Command::Lpop("EmptyList".to_string(), 1);
        let result = lpop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::List(vec![]));
        assert!(store.list_db.get("EmptyList").is_some());
    }

    #[test]
    fn lpop_empty_list_with_0() {
        let mut store = DataStore::new();
        store.list_db.insert("EmptyList".to_string(), vec![]);

        let lpop_cmd = Command::Lpop("EmptyList".to_string(), 0);
        let result = lpop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::List(vec![]));
        assert!(store.list_db.get("EmptyList").is_some());
    }

    #[test]
    fn lpop_list_with_one_item_0_arg() {
        let mut store = DataStore::new();
        store
            .list_db
            .insert("Ashe".to_string(), vec!["B.O.B".to_string()]);

        let lpop_cmd = Command::Lpop("Ashe".to_string(), 0);
        let result = lpop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::List(vec![]));
        assert_eq!(store.list_db.get("Ashe").unwrap().len(), 1);
    }

    #[test]
    fn lpop_list_with_one_item_more_than_1_arg() {
        let mut store = DataStore::new();
        store
            .list_db
            .insert("Ashe".to_string(), vec!["B.O.B".to_string()]);

        let lpop_cmd = Command::Lpop("Ashe".to_string(), 1);
        let result = lpop_cmd.execute_write(&mut store);

        assert_eq!(
            result.unwrap(),
            ResponseType::List(vec!["B.O.B".to_string()])
        );
        assert_eq!(store.list_db.get("Ashe").unwrap().len(), 0);
    }

    #[test]
    fn lpop_list_with_multiple_items_zero_arg() {
        let mut store = set_up_data_store_with_multiple_items_list();

        let lpop_cmd = Command::Lpop("DPS".to_string(), 0);
        let result = lpop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::List(vec![]));
        assert_eq!(store.list_db.get("DPS").unwrap().len(), 5);
    }

    #[test]
    fn lpop_list_with_multiple_items_mid_arg() {
        let mut store = set_up_data_store_with_multiple_items_list();

        let lpop_cmd = Command::Lpop("DPS".to_string(), 3);
        let result = lpop_cmd.execute_write(&mut store);

        assert_eq!(
            result.unwrap(),
            ResponseType::List(vec![
                "Ashe".to_string(),
                "F.R.E.D".to_string(),
                "B.O.B".to_string()
            ])
        );
        assert_eq!(store.list_db.get("DPS").unwrap().len(), 2);
        if let Some(list) = store.list_db.get("DPS") {
            assert!(list.contains(&"Torbjorn".to_string()));
            assert!(list.contains(&"Echo".to_string()));
        }
    }

    #[test]
    fn lpop_list_with_multiple_items_large_arg() {
        let mut store = set_up_data_store_with_multiple_items_list();

        let lpop_cmd = Command::Lpop("DPS".to_string(), 50);
        let result = lpop_cmd.execute_write(&mut store);

        assert_eq!(
            result.unwrap(),
            ResponseType::List(vec![
                "Ashe".to_string(),
                "F.R.E.D".to_string(),
                "B.O.B".to_string(),
                "Torbjorn".to_string(),
                "Echo".to_string(),
            ])
        );
        assert_eq!(store.list_db.get("DPS").unwrap().len(), 0);
    }

    #[test]
    fn lpop_wrongtype_str_with_0_arg() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("WrongTypeStr".to_string(), "NotAList".to_string());

        let lpop_cmd = Command::Lpop("WrongTypeStr".to_string(), 0);
        let result = lpop_cmd.execute_write(&mut store);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn lpop_wrongtype_str_with_more_than_1_arg() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("WrongTypeStr".to_string(), "NotAList".to_string());

        let lpop_cmd = Command::Lpop("WrongTypeStr".to_string(), 10);
        let result = lpop_cmd.execute_write(&mut store);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn lpop_wrongtype_set_with_0_arg() {
        let mut store = set_up_data_store_with_multiple_items_list();

        let lpop_cmd = Command::Lpop("Maps".to_string(), 0);
        let result = lpop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::Null(None));
    }

    #[test]
    fn lpop_wrongtype_set_with_more_than_1_arg() {
        let mut store = set_up_data_store_with_multiple_items_list();

        let lpop_cmd = Command::Lpop("Maps".to_string(), 10);
        let result = lpop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::Null(None));
    }

    #[test]
    fn lpop_nonexistent_key_with_0_arg() {
        let mut store = DataStore::new();
        let lpop_cmd = Command::Lpop("NonExistentKey".to_string(), 0);
        let result = lpop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::Null(None));
    }

    #[test]
    fn lpop_nonexistent_key_with_more_than_1_arg() {
        let mut store = DataStore::new();
        let lpop_cmd = Command::Lpop("NonExistentKey".to_string(), 10);
        let result = lpop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::Null(None));
    }

    /* LPUSH */

    #[test]
    fn lpush_works_for_an_list_that_already_exists() {
        let mut store = DataStore::new();

        // Crear una lista inicial con algunos elementos
        store.list_db.insert(
            "DPS".to_string(),
            vec![
                "Ashe".to_string(),
                "F.R.E.D".to_string(),
                "B.O.B".to_string(),
            ],
        );

        // Ejecutar el comando Lpush para agregar un elemento al principio
        let lpush_cmd = Command::Lpush("DPS".to_string(), vec!["DVA".to_string()]);
        let result = lpush_cmd.execute_write(&mut store);

        // Verificar que el resultado sea la longitud de la lista después de la operación
        assert_eq!(result.unwrap(), ResponseType::Int(4));

        // Verificar que los elementos se hayan insertado correctamente
        if let Some(list) = store.list_db.get("DPS") {
            assert_eq!(list.len(), 4);
            assert_eq!(list[0], "DVA".to_string());
            assert_eq!(list[1], "Ashe".to_string());
            assert_eq!(list[2], "F.R.E.D".to_string());
            assert_eq!(list[3], "B.O.B".to_string());
        }
    }

    /* LRANGE */

    #[test]
    fn lrange_empty_list() {
        let mut store = DataStore::new();
        let lrange_cmd = Command::Lrange("DPS".to_string(), 0, -1);
        let result = lrange_cmd.execute_read(&mut store, None, None, None, None, None);

        match result.unwrap() {
            ResponseType::List(list) => assert_eq!(list.len(), 0),
            _ => assert!(false, "Se esperaba un List vacío"),
        }
    }

    #[test]
    fn lrange_only_one_element_list() {
        let mut store = DataStore::new();
        store
            .list_db
            .insert("DPS".to_string(), vec!["Ashe".to_string()]);

        let lrange_cmd = Command::Lrange("DPS".to_string(), 0, -1);
        let result = lrange_cmd.execute_read(&mut store, None, None, None, None, None);
        assert_eq!(store.list_db.len(), 1);
        match result.unwrap() {
            ResponseType::List(list) => {
                assert_eq!(list.len(), 1);
                assert_eq!(list[0], "Ashe".to_string());
            }
            _ => assert!(false, "Se esperaba un List con 1 elemento"),
        }
    }

    #[test]
    fn lrange_multiple_elements_list() {
        let mut store = DataStore::new();
        store.list_db.insert(
            "DPS".to_string(),
            vec![
                "Ashe".to_string(),
                "F.R.E.D".to_string(),
                "B.O.B".to_string(),
            ],
        );

        let lrange_cmd = Command::Lrange("DPS".to_string(), 0, -1);
        let result = lrange_cmd.execute_read(&mut store, None, None, None, None, None);
        assert_eq!(store.list_db.len(), 1);
        match result.unwrap() {
            ResponseType::List(list) => {
                assert_eq!(list.len(), 3);
                assert_eq!(list[0], "Ashe".to_string());
                assert_eq!(list[1], "F.R.E.D".to_string());
                assert_eq!(list[2], "B.O.B".to_string());
            }
            _ => assert!(false, "Se esperaba un List con 3 elementos"),
        }
    }

    #[test]
    fn lrange_multiple_elements_list_only_reduced_slice() {
        let mut store = set_up_data_store_with_multiple_items_list();
        let lrange_cmd = Command::Lrange("DPS".to_string(), 1, 3);
        let result = lrange_cmd.execute_read(&mut store, None, None, None, None, None);
        assert_eq!(store.list_db.len(), 1);
        match result.unwrap() {
            ResponseType::List(list) => {
                assert_eq!(list.len(), 3);
                assert_eq!(list[0], "F.R.E.D".to_string());
                assert_eq!(list[1], "B.O.B".to_string());
                assert_eq!(list[2], "Torbjorn".to_string());
            }
            _ => assert!(false, "Se esperaba un List de 3 elementos"),
        }
    }

    #[test]
    fn lrange_reduced_slice_from_start() {
        let mut store = set_up_data_store_with_multiple_items_list();
        let lrange_cmd = Command::Lrange("DPS".to_string(), 0, 3);
        let result = lrange_cmd.execute_read(&mut store, None, None, None, None, None);
        assert_eq!(store.list_db.len(), 1);
        match result.unwrap() {
            ResponseType::List(list) => {
                assert_eq!(list.len(), 4);
                assert_eq!(list[0], "Ashe".to_string());
                assert_eq!(list[1], "F.R.E.D".to_string());
                assert_eq!(list[2], "B.O.B".to_string());
                assert_eq!(list[3], "Torbjorn".to_string());
            }
            _ => assert!(false, "Se esperaba un List de 4 elementos"),
        }
    }

    #[test]
    fn lrange_reduced_slice_until_end() {
        let mut store = set_up_data_store_with_multiple_items_list();
        let lrange_cmd = Command::Lrange("DPS".to_string(), 3, -1);
        let result = lrange_cmd.execute_read(&mut store, None, None, None, None, None);
        assert_eq!(store.list_db.len(), 1);
        match result.unwrap() {
            ResponseType::List(list) => {
                assert_eq!(list.len(), 2);
                assert_eq!(list[0], "Torbjorn".to_string());
                assert_eq!(list[1], "Echo".to_string());
            }
            _ => assert!(false, "Se esperaba un List de 2 elementos"),
        }
    }

    #[test]
    fn lrange_out_of_bound_lower_limit() {
        let mut store = set_up_data_store_with_multiple_items_list();
        let lrange_cmd = Command::Lrange("DPS".to_string(), -1, -1);
        let result = lrange_cmd.execute_read(&mut store, None, None, None, None, None);
        assert_eq!(store.list_db.len(), 1);
        match result.unwrap() {
            ResponseType::List(list) => {
                assert_eq!(list.len(), 1);
                assert_eq!(list[0], "Echo".to_string());
            }
            _ => assert!(false, "Se esperaba un List con 1 elemento"),
        }
    }

    #[test]
    fn lrange_out_of_bound_higher_upper_limit() {
        let mut store = set_up_data_store_with_multiple_items_list();
        let lrange_cmd = Command::Lrange("DPS".to_string(), 100, -1);
        let result = lrange_cmd.execute_read(&mut store, None, None, None, None, None);
        assert_eq!(store.list_db.len(), 1);
        match result.unwrap() {
            ResponseType::List(list) => {
                assert_eq!(list.len(), 0);
            }
            _ => assert!(false, "Se esperaba un List vacío"),
        }
    }

    #[test]
    fn lrange_out_of_bound_upper_limit() {
        let mut store = set_up_data_store_with_multiple_items_list();
        let lrange_cmd = Command::Lrange("DPS".to_string(), 1, 100);
        let result = lrange_cmd.execute_read(&mut store, None, None, None, None, None);
        assert_eq!(store.list_db.len(), 1);
        match result.unwrap() {
            ResponseType::List(list) => {
                assert_eq!(list.len(), 4);
                assert_eq!(list[0], "F.R.E.D".to_string());
                assert_eq!(list[1], "B.O.B".to_string());
                assert_eq!(list[2], "Torbjorn".to_string());
                assert_eq!(list[3], "Echo".to_string());
            }
            _ => assert!(false, "Se esperaba un List de 4 elementos"),
        }
    }

    #[test]
    fn lrange_out_of_bound_both_limits_repectively() {
        let mut store = set_up_data_store_with_multiple_items_list();
        let lrange_cmd = Command::Lrange("DPS".to_string(), -10, 100);
        let result = lrange_cmd.execute_read(&mut store, None, None, None, None, None);
        assert_eq!(store.list_db.len(), 1);
        match result.unwrap() {
            ResponseType::List(list) => {
                assert_eq!(list.len(), 5);
                assert_eq!(list[0], "Ashe".to_string());
                assert_eq!(list[1], "F.R.E.D".to_string());
                assert_eq!(list[2], "B.O.B".to_string());
                assert_eq!(list[3], "Torbjorn".to_string());
                assert_eq!(list[4], "Echo".to_string());
            }
            _ => assert!(false, "Se esperaba un List de 5 elementos"),
        }
    }

    #[test]
    fn lrange_both_are_higher_than_len() {
        let mut store = set_up_data_store_with_multiple_items_list();
        let lrange_cmd = Command::Lrange("DPS".to_string(), 100, 100);
        let result = lrange_cmd.execute_read(&mut store, None, None, None, None, None);
        assert_eq!(store.list_db.len(), 1);
        match result.unwrap() {
            ResponseType::List(list) => {
                assert_eq!(list.len(), 0);
            }
            _ => assert!(false, "Se esperaba un List vacío"),
        }
    }

    #[test]
    fn lrange_lower_limit_is_higher_than_higher_limit() {
        let mut store = set_up_data_store_with_multiple_items_list();
        let lrange_cmd = Command::Lrange("DPS".to_string(), 4, 3);
        let result = lrange_cmd.execute_read(&mut store, None, None, None, None, None);
        assert_eq!(store.list_db.len(), 1);
        match result.unwrap() {
            ResponseType::List(list) => {
                assert_eq!(list.len(), 0);
            }
            _ => assert!(false, "Se esperaba un List vacío"),
        }
    }

    #[test]
    fn lrange_both_limits_are_equal() {
        let mut store = set_up_data_store_with_multiple_items_list();
        let lrange_cmd = Command::Lrange("DPS".to_string(), 4, 4);
        let result = lrange_cmd.execute_read(&mut store, None, None, None, None, None);
        assert_eq!(store.list_db.len(), 1);
        match result.unwrap() {
            ResponseType::List(list) => {
                assert_eq!(list.len(), 1);
                assert_eq!(list[0], "Echo".to_string());
            }
            _ => assert!(false, "Se esperaba un List con 1 elemento"),
        }
    }

    #[test]
    fn lrange_doesnt_work_for_a_set_string() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("DPS".to_string(), "Soldier:76".to_string());
        let lrange_cmd = Command::Lrange("DPS".to_string(), 0, -1);
        let result = lrange_cmd.execute_read(&mut store, None, None, None, None, None);
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    /* RPOP */

    #[test]
    fn rpop_empty_list() {
        let mut store = DataStore::new();
        store.list_db.insert("EmptyList".to_string(), vec![]);

        let rpop_cmd = Command::Rpop("EmptyList".to_string(), 1);
        let result = rpop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::List(vec![]));
        assert!(store.list_db.get("EmptyList").is_some());
    }

    #[test]
    fn rpop_empty_list_with_0() {
        let mut store = DataStore::new();
        store.list_db.insert("EmptyList".to_string(), vec![]);

        let rpop_cmd = Command::Rpop("EmptyList".to_string(), 0);
        let result = rpop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::List(vec![]));
        assert!(store.list_db.get("EmptyList").is_some());
    }

    #[test]
    fn rpop_list_with_one_item_0_arg() {
        let mut store = DataStore::new();
        store
            .list_db
            .insert("Ashe".to_string(), vec!["B.O.B".to_string()]);

        let rpop_cmd = Command::Rpop("Ashe".to_string(), 0);
        let result = rpop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::List(vec![]));
        assert_eq!(store.list_db.get("Ashe").unwrap().len(), 1);
    }

    #[test]
    fn rpop_list_with_one_item_more_than_1_arg() {
        let mut store = DataStore::new();
        store
            .list_db
            .insert("Ashe".to_string(), vec!["B.O.B".to_string()]);

        let rpop_cmd = Command::Rpop("Ashe".to_string(), 1);
        let result = rpop_cmd.execute_write(&mut store);

        assert_eq!(
            result.unwrap(),
            ResponseType::List(vec!["B.O.B".to_string()])
        );
        assert_eq!(store.list_db.get("Ashe").unwrap().len(), 0);
    }

    #[test]
    fn rpop_list_with_multiple_items_zero_arg() {
        let mut store = set_up_data_store_with_multiple_items_list();

        let rpop_cmd = Command::Rpop("DPS".to_string(), 0);
        let result = rpop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::List(vec![]));
        assert_eq!(store.list_db.get("DPS").unwrap().len(), 5);
    }

    #[test]
    fn rpop_list_with_multiple_items_mid_arg() {
        let mut store = set_up_data_store_with_multiple_items_list();

        let rpop_cmd = Command::Rpop("DPS".to_string(), 3);
        let result = rpop_cmd.execute_write(&mut store);

        assert_eq!(
            result.unwrap(),
            ResponseType::List(vec![
                "Echo".to_string(),
                "Torbjorn".to_string(),
                "B.O.B".to_string()
            ])
        );
        assert_eq!(store.list_db.get("DPS").unwrap().len(), 2);
        if let Some(list) = store.list_db.get("DPS") {
            assert!(list.contains(&"Ashe".to_string()));
            assert!(list.contains(&"F.R.E.D".to_string()));
        }
    }

    #[test]
    fn rpop_list_with_multiple_items_large_arg() {
        let mut store = set_up_data_store_with_multiple_items_list();

        let rpop_cmd = Command::Rpop("DPS".to_string(), 50);
        let result = rpop_cmd.execute_write(&mut store);

        assert_eq!(
            result.unwrap(),
            ResponseType::List(vec![
                "Echo".to_string(),
                "Torbjorn".to_string(),
                "B.O.B".to_string(),
                "F.R.E.D".to_string(),
                "Ashe".to_string(),
            ])
        );
        assert_eq!(store.list_db.get("DPS").unwrap().len(), 0);
    }

    #[test]
    fn rpop_wrongtype_str_with_0_arg() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("WrongTypeStr".to_string(), "NotAList".to_string());

        let rpop_cmd = Command::Rpop("WrongTypeStr".to_string(), 0);
        let result = rpop_cmd.execute_write(&mut store);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn rpop_wrongtype_str_with_more_than_1_arg() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("WrongTypeStr".to_string(), "NotAList".to_string());

        let rpop_cmd = Command::Rpop("WrongTypeStr".to_string(), 10);
        let result = rpop_cmd.execute_write(&mut store);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn rpop_wrongtype_set_with_0_arg() {
        let mut store = set_up_data_store_with_multiple_items_set();

        let rpop_cmd = Command::Rpop("Maps".to_string(), 0);
        let result = rpop_cmd.execute_write(&mut store);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn rpop_wrongtype_set_with_more_than_1_arg() {
        let mut store = set_up_data_store_with_multiple_items_set();

        let rpop_cmd = Command::Rpop("Maps".to_string(), 10);
        let result = rpop_cmd.execute_write(&mut store);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn rpop_nonexistent_key_with_0_arg() {
        let mut store = DataStore::new();
        let rpop_cmd = Command::Rpop("NonExistentKey".to_string(), 0);
        let result = rpop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::Null(None));
    }

    #[test]
    fn rpop_nonexistent_key_with_more_than_1_arg() {
        let mut store = DataStore::new();
        let rpop_cmd = Command::Rpop("NonExistentKey".to_string(), 10);
        let result = rpop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::Null(None));
    }

    /* RPUSH */

    #[test]
    fn rpush_with_no_previous_items_works() {
        let mut store = DataStore::new();
        let rpush_cmd = Command::Rpush("TANKS".to_string(), vec!["DVA".to_string()]);
        let result = rpush_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::Int(1));
        assert!(store.string_db.is_empty());
        assert!(store.set_db.is_empty());
        let list = store.list_db.get("TANKS").expect("Debe existir la lista");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], "DVA".to_string());
    }

    #[test]
    fn rpush_with_previous_items_works() {
        let mut store = DataStore::new();
        store
            .list_db
            .insert("TANKS".to_string(), vec!["DVA".to_string()]);
        let rpush_cmd = Command::Rpush(
            "TANKS".to_string(),
            vec!["Reinhardt".to_string(), "Orisa".to_string()],
        );
        let result = rpush_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::Int(3));
        assert_eq!(store.list_db.len(), 1);
        let list = store.list_db.get("TANKS").expect("Debe existir la lista");
        assert_eq!(list.len(), 3);
        assert_eq!(list[0], "DVA".to_string());
        assert_eq!(list[1], "Reinhardt".to_string());
        assert_eq!(list[2], "Orisa".to_string());
    }

    #[test]
    fn rpush_doesnt_work_after_using_a_set_command() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("SUPPORT".to_string(), "Kiriko".to_string());

        let rpush_cmd = Command::Rpush(
            "SUPPORT".to_string(),
            vec!["Ana".to_string(), "Moira".to_string()],
        );
        let result = rpush_cmd.execute_write(&mut store);
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
        assert_eq!(store.string_db.len(), 1);
        assert_eq!(store.string_db.get("SUPPORT").unwrap(), "Kiriko");
    }

    /* SET TESTS */

    /* SADD */

    #[test]
    fn sadd_creates_a_set() {
        let mut store = DataStore::new();
        let set_cmd = Command::Sadd(
            "Maps".to_string(),
            vec!["King's Row".to_string(), "Gilbraltar".to_string()],
        );
        let result = set_cmd.execute_write(&mut store);

        // La función debe retornar la cantidad de elementos insertados.
        assert_eq!(result.unwrap(), ResponseType::Int(2));

        // Ahora se espera que "Maps" aparezca en el contenedor de sets.
        assert_eq!(store.set_db.len(), 1);
        let set = store.set_db.get("Maps").expect("Debe existir el set");
        let mut aux = HashSet::new();
        aux.insert("King's Row".to_string());
        aux.insert("Gilbraltar".to_string());
        assert_eq!(set, &aux);
    }

    #[test]
    fn sadd_adds_to_current_set() {
        let mut store = DataStore::new();
        store.set_db.insert(
            "Maps".to_string(),
            HashSet::from(["King's Row".to_string(), "Gilbraltar".to_string()]),
        );

        let set_cmd = Command::Sadd("Maps".to_string(), vec!["Antartica".to_string()]);
        let result = set_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::Int(1));
        // Se espera que el set tenga 3 elementos.
        let mut aux = HashSet::new();
        aux.insert("King's Row".to_string());
        aux.insert("Gilbraltar".to_string());
        aux.insert("Antartica".to_string());

        assert_eq!(store.set_db.len(), 1);
        let set = store.set_db.get("Maps").expect("Debe existir el set");
        assert_eq!(set.len(), 3);
        for expected in aux {
            assert!(set.contains(&expected));
        }
    }

    #[test]
    fn sadd_doesnt_work_over_set_strings() {
        let mut store = DataStore::new();
        // Primero, se inserta un STRING con el comando SET en lugar de un set.
        store
            .string_db
            .insert("Ashe".to_string(), "B.O.B".to_string());

        let sadd_cmd = Command::Sadd("Ashe".to_string(), vec!["F.R.E.D".to_string()]);
        let result_sadd = sadd_cmd.execute_write(&mut store);

        assert!(matches!(result_sadd.unwrap_err(), CommandError::WrongType));
        // La llave "Ashe" debe seguir en string_db.
        assert_eq!(store.string_db.len(), 1);
        assert_eq!(store.string_db.get("Ashe").unwrap(), "B.O.B");
    }

    #[test]
    fn sadd_doesnt_work_over_lists() {
        let mut store = DataStore::new();
        // Insertamos una lista en "Ashe" mediante RPUSH.
        store
            .list_db
            .insert("Ashe".to_string(), vec!["B.O.B".to_string()]);

        let sadd_cmd = Command::Sadd("Ashe".to_string(), vec!["F.R.E.D".to_string()]);
        let result_sadd = sadd_cmd.execute_write(&mut store);

        assert!(matches!(result_sadd.unwrap_err(), CommandError::WrongType));
        // "Ashe" debe permanecer en el contenedor de listas.
        assert_eq!(store.list_db.len(), 1);
        let list = store.list_db.get("Ashe").unwrap();
        assert_eq!(list, &vec!["B.O.B".to_string()]);
    }

    /* SCARD */

    #[test]
    fn scard_works_over_no_set() {
        let mut store = DataStore::new();
        let scard_cmd = Command::Scard("Maps".to_string());
        let result = scard_cmd.execute_read(&mut store, None, None, None, None, None);

        // Al no existir el set se retorna 0.
        assert_eq!(result.unwrap(), ResponseType::Int(0));
    }

    #[test]
    fn scard_works_over_one_item_set() {
        let mut store = DataStore::new();
        store.set_db.insert(
            "Genji".to_string(),
            HashSet::from(["I need healing".to_string()]),
        );

        let scard_cmd = Command::Scard("Genji".to_string());
        let result = scard_cmd.execute_read(&mut store, None, None, None, None, None);

        assert_eq!(result.unwrap(), ResponseType::Int(1));

        let set = store.set_db.get("Genji").unwrap();
        assert_eq!(set.len(), 1);
        assert!(set.contains("I need healing"));
    }

    #[test]
    fn scard_works_over_multiple_items_set() {
        let mut store = DataStore::new();
        store.set_db.insert(
            "Maps".to_string(),
            HashSet::from([
                "El Dorado".to_string(),
                "Petra".to_string(),
                "Busan".to_string(),
            ]),
        );

        let scard_cmd = Command::Scard("Maps".to_string());
        let result = scard_cmd.execute_read(&mut store, None, None, None, None, None);

        assert_eq!(result.unwrap(), ResponseType::Int(3));

        let expected: HashSet<String> = ["El Dorado", "Petra", "Busan"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let set = store.set_db.get("Maps").unwrap();
        assert_eq!(set, &expected);
    }

    #[test]
    fn scard_doesnt_work_over_set_strings() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Hammond".to_string(), "Ball".to_string());

        let scard_cmd = Command::Scard("Hammond".to_string());
        let result = scard_cmd.execute_read(&mut store, None, None, None, None, None);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
        // "Hammond" debe permanecer en string_db.
        assert_eq!(store.string_db.get("Hammond").unwrap(), "Ball");
    }

    #[test]
    fn scard_doesnt_work_over_lists() {
        let mut store = DataStore::new();
        store
            .list_db
            .insert("Hammond".to_string(), vec!["Ball".to_string()]);

        let scard_cmd = Command::Scard("Hammond".to_string());
        let result = scard_cmd.execute_read(&mut store, None, None, None, None, None);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    /* SISMEMBER */

    #[test]
    fn sismember_works_for_non_existent_set() {
        let mut store = DataStore::new();
        let sismemeber_cmd = Command::Sismember("Game modes".to_string(), "Archives".to_string());
        let result = sismemeber_cmd.execute_read(&mut store, None, None, None, None, None);

        // Al no existir la clave, se retorna 0.
        assert_eq!(result.unwrap(), ResponseType::Int(0));
    }

    #[test]
    fn sismember_works_for_non_existent_value() {
        let mut store = DataStore::new();
        store.set_db.insert(
            "Maps".to_string(),
            HashSet::from([
                "El Dorado".to_string(),
                "Petra".to_string(),
                "Busan".to_string(),
            ]),
        );

        let sismemeber_cmd = Command::Sismember("Maps".to_string(), "Gilbraltar".to_string());
        let result = sismemeber_cmd.execute_read(&mut store, None, None, None, None, None);

        // Se espera 0 ya que "Gilbraltar" no está en el set.
        assert_eq!(result.unwrap(), ResponseType::Int(0));

        let expected: HashSet<String> = ["El Dorado", "Petra", "Busan"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let set = store.set_db.get("Maps").unwrap();
        assert_eq!(set, &expected);
    }

    #[test]
    fn sismember_works_for_one_item_set() {
        let mut store = DataStore::new();
        store
            .set_db
            .insert("Maps".to_string(), HashSet::from(["El Dorado".to_string()]));

        let sismember_cmd = Command::Sismember("Maps".to_string(), "El Dorado".to_string());
        let result = sismember_cmd.execute_read(&mut store, None, None, None, None, None);

        assert_eq!(result.unwrap(), ResponseType::Int(1));

        let expected: HashSet<String> = ["El Dorado"].iter().map(|s| s.to_string()).collect();
        let set = store.set_db.get("Maps").unwrap();
        assert_eq!(set, &expected);
    }

    #[test]
    fn sismember_works_for_multiple_items_set() {
        let mut store = DataStore::new();
        store.set_db.insert(
            "Maps".to_string(),
            HashSet::from([
                "El Dorado".to_string(),
                "Petra".to_string(),
                "Busan".to_string(),
            ]),
        );

        let sismember_cmd = Command::Sismember("Maps".to_string(), "Petra".to_string());
        let result = sismember_cmd.execute_read(&mut store, None, None, None, None, None);

        assert_eq!(result.unwrap(), ResponseType::Int(1));

        let expected: HashSet<String> = ["El Dorado", "Petra", "Busan"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let set = store.set_db.get("Maps").unwrap();
        assert_eq!(set, &expected);
    }

    #[test]
    fn sismember_works_for_multiple_items_set_at_beggining() {
        let mut store = DataStore::new();
        store.set_db.insert(
            "Maps".to_string(),
            HashSet::from([
                "El Dorado".to_string(),
                "Petra".to_string(),
                "Busan".to_string(),
            ]),
        );

        let sismember_cmd = Command::Sismember("Maps".to_string(), "El Dorado".to_string());
        let result = sismember_cmd.execute_read(&mut store, None, None, None, None, None);

        assert_eq!(result.unwrap(), ResponseType::Int(1));

        let expected: HashSet<String> = ["El Dorado", "Petra", "Busan"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let set = store.set_db.get("Maps").unwrap();
        assert_eq!(set, &expected);
    }

    #[test]
    fn sismember_works_for_multiple_items_set_at_end() {
        let mut store = DataStore::new();
        store.set_db.insert(
            "Maps".to_string(),
            HashSet::from([
                "El Dorado".to_string(),
                "Petra".to_string(),
                "Busan".to_string(),
            ]),
        );

        let sismember_cmd = Command::Sismember("Maps".to_string(), "Busan".to_string());
        let result = sismember_cmd.execute_read(&mut store, None, None, None, None, None);

        assert_eq!(result.unwrap(), ResponseType::Int(1));

        let expected: HashSet<String> = ["El Dorado", "Petra", "Busan"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let set = store.set_db.get("Maps").unwrap();
        assert_eq!(set, &expected);
    }

    #[test]
    fn sismember_doesnt_work_for_set_strings() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Mei".to_string(), "Iceberg".to_string());

        let sismember_cmd = Command::Sismember("Mei".to_string(), "Iceberg".to_string());
        let result = sismember_cmd.execute_read(&mut store, None, None, None, None, None);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
        // "Mei" debe permanecer en string_db.
        assert_eq!(store.string_db.get("Mei").unwrap(), "Iceberg");
    }

    #[test]
    fn sismember_doesnt_work_for_lists() {
        let mut store = DataStore::new();
        // Insertar una lista en "DPS" por ejemplo.
        store.list_db.insert(
            "DPS".to_string(),
            vec![
                "Ashe".to_string(),
                "F.R.E.D".to_string(),
                "B.O.B".to_string(),
                "Torbjorn".to_string(),
                "Echo".to_string(),
            ],
        );

        let sismember_cmd = Command::Sismember("DPS".to_string(), "F.R.E.D".to_string());
        let result = sismember_cmd.execute_read(&mut store, None, None, None, None, None);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
        // La clave "DPS" debe seguir en list_db y sin cambios.
        let list = store.list_db.get("DPS").unwrap();
        assert_eq!(list.len(), 5);
        assert_eq!(list[0], "Ashe".to_string());
        assert_eq!(list[1], "F.R.E.D".to_string());
        assert_eq!(list[2], "B.O.B".to_string());
        assert_eq!(list[3], "Torbjorn".to_string());
        assert_eq!(list[4], "Echo".to_string());
    }

    /* SMEMBERS */

    #[test]
    fn smembers_works_properly_over_an_empty_set() {
        let mut store = DataStore::new();
        let smem_cmd = Command::Smembers("Winton".to_string());
        let result = smem_cmd.execute_read(&mut store, None, None, None, None, None);

        // Al no existir la clave "Winton" se devuelve un set vacío.
        assert_eq!(result.unwrap(), ResponseType::Set(HashSet::new()));
    }

    #[test]
    fn smembers_works_properly_over_one_item_set() {
        let mut store = DataStore::new();
        store
            .set_db
            .insert("Winton".to_string(), HashSet::from(["Honey".to_string()]));

        let smem_cmd = Command::Smembers("Winton".to_string());
        let result = smem_cmd.execute_read(&mut store, None, None, None, None, None);

        match result.unwrap() {
            ResponseType::Set(set) => {
                assert_eq!(set.len(), 1);
                assert!(set.contains("Honey"));
            }
            _ => assert!(false, "Se esperaba un ResponseType::Set"),
        }
        // Se verifica internamente
        assert_eq!(store.set_db.len(), 1);
        let set = store.set_db.get("Winton").unwrap();
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn smembers_works_properly_over_multiple_items_set() {
        let mut store = DataStore::new();
        store.set_db.insert(
            "Winton".to_string(),
            HashSet::from(["Honey".to_string(), "Glasses".to_string()]),
        );

        let smem_cmd = Command::Smembers("Winton".to_string());
        let result = smem_cmd.execute_read(&mut store, None, None, None, None, None);

        match result.unwrap() {
            ResponseType::Set(set) => {
                assert_eq!(set.len(), 2);
                assert!(set.contains("Honey"));
                assert!(set.contains("Glasses"));
            }
            _ => assert!(false, "Se esperaba un ResponseType::Set"),
        }
        // Verificamos el estado interno.
        let set = store.set_db.get("Winton").unwrap();
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn smembers_doesnt_work_over_set_strings() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Ashe".to_string(), "B.O.B".to_string());

        let smem_cmd = Command::Smembers("Ashe".to_string());
        let result = smem_cmd.execute_read(&mut store, None, None, None, None, None);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
        // "Ashe" debe seguir en el contenedor de strings.
        assert_eq!(store.string_db.len(), 1);
        assert_eq!(store.string_db.get("Ashe").unwrap(), "B.O.B");
    }

    #[test]
    fn smembers_doesnt_work_over_lists() {
        let mut store = DataStore::new();
        // Inserta una lista en "Maps" por medio de RPUSH.
        store
            .list_db
            .insert("Maps".to_string(), vec!["Oasis".to_string()]);

        let smem_cmd = Command::Smembers("Maps".to_string());
        let result = smem_cmd.execute_read(&mut store, None, None, None, None, None);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
        // La clave "Maps" debe permanecer en list_db.
        let list = store.list_db.get("Maps").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], "Oasis".to_string());
    }

    /* SMOVE */

    #[test]
    fn smove_works_for_an_empty_set() {
        let mut store = DataStore::new();
        let smove_cmd = Command::SMove(
            "Maps".to_string(),
            "Maps2".to_string(),
            "El Dorado".to_string(),
        );
        let result = smove_cmd.execute_write(&mut store);

        // Al no existir el set "Maps", no se mueve nada.
        assert_eq!(store.set_db.len(), 0);
        assert_eq!(result.unwrap(), ResponseType::Int(0));
    }

    #[test]
    fn smove_works_for_an_non_empty_set() {
        let mut store = DataStore::new();

        // Crear el conjunto de origen con 3 elementos.
        store.set_db.insert(
            "SourceSet".to_string(),
            HashSet::from([
                "El Dorado".to_string(),
                "Petra".to_string(),
                "Busan".to_string(),
            ]),
        );

        // Crear el conjunto de destino vacío.
        store
            .set_db
            .insert("DestinationSet".to_string(), HashSet::new());

        // Mover "Petra" de SourceSet a DestinationSet.
        let smove_cmd = Command::SMove(
            "SourceSet".to_string(),
            "DestinationSet".to_string(),
            "Petra".to_string(),
        );
        let result = smove_cmd.execute_write(&mut store);

        // Se espera que se mueva 1 (exitosamente).
        assert_eq!(result.unwrap(), ResponseType::Int(1));

        // Verificar que "Petra" ya no se encuentre en SourceSet.
        let source_set = store
            .set_db
            .get("SourceSet")
            .expect("Debe existir SourceSet");
        assert_eq!(source_set.len(), 2);
        assert!(source_set.contains("El Dorado"));
        assert!(source_set.contains("Busan"));
        assert!(!source_set.contains("Petra"));

        // Verificar que "Petra" se haya insertado en DestinationSet.
        let dest_set = store
            .set_db
            .get("DestinationSet")
            .expect("Debe existir DestinationSet");
        assert_eq!(dest_set.len(), 1);
        assert!(dest_set.contains("Petra"));
    }

    #[test]
    fn smove_doesnt_work_for_both_src_and_dst_strings() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Hammond".to_string(), "Ball".to_string());
        store
            .string_db
            .insert("Winton".to_string(), "Honey".to_string());
        let smove_cmd = Command::SMove(
            "Hammond".to_string(),
            "Winton".to_string(),
            "Ball".to_string(),
        );
        let result = smove_cmd.execute_write(&mut store);
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn smove_doesnt_work_for_src_string() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Hammond".to_string(), "Ball".to_string());
        let mut aux = HashSet::new();
        aux.insert("Glasses".to_string());
        aux.insert("Honey".to_string());
        store.set_db.insert("Winton".to_string(), aux);

        let smove_cmd = Command::SMove(
            "Hammond".to_string(),
            "Winton".to_string(),
            "Hammond".to_string(),
        );
        let result = smove_cmd.execute_write(&mut store);
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn smove_doesnt_work_for_dst_string() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Hammond".to_string(), "Ball".to_string());
        let mut aux = HashSet::new();
        aux.insert("Glasses".to_string());
        aux.insert("Honey".to_string());
        store.set_db.insert("Winton".to_string(), aux);

        let smove_cmd = Command::SMove(
            "Winton".to_string(),
            "Hammond".to_string(),
            "Honey".to_string(),
        );
        let result = smove_cmd.execute_write(&mut store);
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn smove_doesnt_work_for_both_src_and_dst_lists() {
        let mut store = DataStore::new();
        store.list_db.insert("Hammond".to_string(), vec![]);
        store.list_db.insert("Winton".to_string(), vec![]);
        let smove_cmd = Command::SMove(
            "Hammond".to_string(),
            "Winton".to_string(),
            "Ball".to_string(),
        );
        let result = smove_cmd.execute_write(&mut store);
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn smove_doesnt_work_for_src_list() {
        let mut store = DataStore::new();
        let mut aux = HashSet::new();
        aux.insert("Ball".to_string());
        store.set_db.insert("Hammond".to_string(), aux);
        store
            .list_db
            .insert("Winton".to_string(), vec!["Glasses".to_string()]);
        let smove_cmd = Command::SMove(
            "Winton".to_string(),
            "Hammond".to_string(),
            "Glasses".to_string(),
        );
        let result = smove_cmd.execute_write(&mut store);
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn smove_doesnt_work_for_dest_list() {
        let mut store = DataStore::new();
        let mut aux = HashSet::new();
        aux.insert("Ball".to_string());
        store.set_db.insert("Hammond".to_string(), aux);
        store.list_db.insert("Winton".to_string(), vec![]);
        let smove_cmd = Command::SMove(
            "Hammond".to_string(),
            "Winton".to_string(),
            "Ball".to_string(),
        );
        let result = smove_cmd.execute_write(&mut store);
        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn smove_works_for_both_non_existent_sets() {
        let mut empty_store = DataStore::new();
        let smove_cmd = Command::SMove(
            "Hammond".to_string(),
            "Winton".to_string(),
            "Ball".to_string(),
        );
        let result = smove_cmd.execute_write(&mut empty_store);
        assert_eq!(result.unwrap(), ResponseType::Int(0));
    }

    #[test]
    fn smove_works_for_non_existent_src_set() {
        let mut store = DataStore::new();
        let mut aux = HashSet::new();
        aux.insert("Ball".to_string());
        store.set_db.insert("Hammond".to_string(), aux);
        let smove_cmd = Command::SMove(
            "Winton".to_string(),
            "Hammond".to_string(),
            "Glasses".to_string(),
        );
        let result = smove_cmd.execute_write(&mut store);
        assert_eq!(result.unwrap(), ResponseType::Int(0));
    }

    #[test]
    fn smove_works_for_non_existent_dst_set() {
        let mut store = DataStore::new();
        let mut aux = HashSet::new();
        aux.insert("Ball".to_string());
        store.set_db.insert("Hammond".to_string(), aux);
        let smove_cmd = Command::SMove(
            "Hammond".to_string(),
            "Winton".to_string(),
            "Glasses".to_string(),
        );
        let result = smove_cmd.execute_write(&mut store);
        assert_eq!(result.unwrap(), ResponseType::Int(0));
    }

    /* SPOP */

    #[test]
    fn spop_empty_set_0_arg() {
        let mut store = DataStore::new();
        let set = HashSet::new();
        store.set_db.insert("Maps".to_string(), set);
        let spop_cmd = Command::Spop("Maps".to_string(), 0);
        let result = spop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::List(vec![]));
        assert!(store.set_db.get("Maps").is_some());
    }

    #[test]
    fn spop_empty_set_bigger_arg() {
        let mut store = DataStore::new();
        let set = HashSet::new();
        store.set_db.insert("Maps".to_string(), set);
        let spop_cmd = Command::Spop("Maps".to_string(), 5);
        let result = spop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::List(vec![]));
        assert!(store.set_db.get("Maps").is_some());
    }

    #[test]
    fn spop_set_with_one_item() {
        let mut store = DataStore::new();
        store
            .set_db
            .insert("DPS".to_string(), HashSet::from(["Soldier:76".to_string()]));

        let spop_cmd = Command::Spop("DPS".to_string(), 1);
        let result = spop_cmd.execute_write(&mut store);

        assert_eq!(
            result.unwrap(),
            ResponseType::List(vec!["Soldier:76".to_string()])
        );
        assert_eq!(store.set_db.get("DPS").unwrap().len(), 0);
    }

    #[test]
    fn spop_set_with_one_item_twice() {
        let mut store = DataStore::new();
        store
            .set_db
            .insert("DPS".to_string(), HashSet::from(["Soldier:76".to_string()]));

        let spop_cmd = Command::Spop("DPS".to_string(), 1);
        let _ = spop_cmd.execute_write(&mut store);

        let spop_cmd_again = Command::Spop("DPS".to_string(), 1);
        let result = spop_cmd_again.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::List(vec![]));
        assert_eq!(store.set_db.get("DPS").unwrap().len(), 0);
    }

    #[test]
    fn spop_set_with_few_items_zero_arg() {
        let mut store = DataStore::new();
        store.set_db.insert(
            "DPS".to_string(),
            HashSet::from([
                "Echo".to_string(),
                "Pharah".to_string(),
                "Sombra".to_string(),
            ]),
        );

        let spop_cmd = Command::Spop("DPS".to_string(), 0);
        let result = spop_cmd.execute_write(&mut store);

        assert_eq!(result.unwrap(), ResponseType::List(vec![]));
        assert_eq!(store.set_db.get("DPS").unwrap().len(), 3);
    }

    #[test]
    fn spop_set_with_few_items_middle_arg() {
        let mut store = set_up_data_store_with_multiple_items_set();

        let spop_cmd = Command::Spop("Maps".to_string(), 2);
        let result = spop_cmd.execute_write(&mut store);

        let result_list = match result.unwrap() {
            ResponseType::List(list) => list,
            _ => panic!("Expected a list response"),
        };

        // No sabés que se va a ir
        assert_eq!(result_list.len(), 2);
        assert_eq!(store.set_db.get("Maps").unwrap().len(), 1);
    }

    #[test]
    fn spop_set_with_few_items_large_arg() {
        let mut store = set_up_data_store_with_multiple_items_set();

        let spop_cmd = Command::Spop("Maps".to_string(), 50);
        let result = spop_cmd.execute_write(&mut store);

        let result_list = match result.unwrap() {
            ResponseType::List(list) => list,
            _ => panic!("Expected a list response"),
        };

        assert_eq!(result_list.len(), 3);
        assert!(result_list.contains(&"El Dorado".to_string()));
        assert!(result_list.contains(&"Petra".to_string()));
        assert!(result_list.contains(&"Busan".to_string()));
        assert_eq!(store.set_db.get("Maps").unwrap().len(), 0);
    }

    #[test]
    fn spop_wrongtype_str() {
        let mut store = DataStore::new();
        store
            .string_db
            .insert("Perú".to_string(), "Illari".to_string());

        let spop_cmd = Command::Spop("Perú".to_string(), 1);
        let result = spop_cmd.execute_write(&mut store);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn spop_wrongtype_list() {
        let mut store = DataStore::new();
        store
            .list_db
            .insert("AUS".to_string(), vec!["Junk*".to_string()]);

        let spop_cmd = Command::Spop("AUS".to_string(), 1);
        let result = spop_cmd.execute_write(&mut store);

        assert!(matches!(result.unwrap_err(), CommandError::WrongType));
    }

    #[test]
    fn spop_nonexistent_key() {
        let mut store = DataStore::new();
        let spop_cmd = Command::Spop("NonExistentKey".to_string(), 1);
        let result = spop_cmd.execute_write(&mut store);
        assert_eq!(result.unwrap(), ResponseType::Null(None));
    }
}
