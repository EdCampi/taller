//! Funciones para leer el dump.rdb y generar un DataStore.

// IMPORTS
use crate::storage::DataStore;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io;
use std::io::Read;

// CONSTANTES
const USIZE_BYTES_SIZE: usize = 8;

// FUNCIONES

/// Lee un entero de 8 bytes. Usado para leer longitudes de
/// tipos de datos dinámicos como HashMaps, HashSets y vectores.
fn read_len<R: Read>(reader: &mut R) -> io::Result<usize> {
    let mut read_bytes = [0u8; USIZE_BYTES_SIZE];
    reader.read_exact(&mut read_bytes)?;
    Ok(usize::from_be_bytes(read_bytes))
}

/// Lee una cadena de caracteres de un archivo.
fn read_string<R: Read>(reader: &mut R) -> io::Result<String> {
    let len = read_len(reader)?;
    let mut str_bytes = vec![0u8; len];
    reader.read_exact(&mut str_bytes)?;
    Ok(String::from_utf8(str_bytes).unwrap())
}

/// Lee un hashmap de strings a strings.
fn read_string_map(ds_src: &mut File, str_db: &mut HashMap<String, String>) -> io::Result<()> {
    let str_db_len = read_len(ds_src)?;
    for _ in 0..str_db_len {
        let key = read_string(ds_src)?;
        let value = read_string(ds_src)?;
        str_db.insert(key, value);
    }
    Ok(())
}

/// Lee un hashmap de strings a vectores de strings.
fn read_list_map(ds_src: &mut File, list_db: &mut HashMap<String, Vec<String>>) -> io::Result<()> {
    let list_db_len = read_len(ds_src)?;
    for _ in 0..list_db_len {
        let key = read_string(ds_src)?;
        let value_len = read_len(ds_src)?;
        let mut value = Vec::new();
        for _ in 0..value_len {
            value.push(read_string(ds_src)?);
        }
        list_db.insert(key, value);
    }
    Ok(())
}

/// Lee un hashmap de strings a hashsets de strings.
fn read_set_map(
    ds_src: &mut File,
    set_db: &mut HashMap<String, HashSet<String>>,
) -> io::Result<()> {
    let set_db_len = read_len(ds_src)?;
    for _ in 0..set_db_len {
        let key = read_string(ds_src)?;
        let value_len = read_len(ds_src)?;
        let mut value = HashSet::new();
        for _ in 0..value_len {
            value.insert(read_string(ds_src)?);
        }
        set_db.insert(key, value);
    }
    Ok(())
}

/// Dado el file dump.rdb, lee el contenido y lo devuelve en un DataStore.
pub fn deserialize_db(path: String) -> Result<DataStore, io::Error> {
    let mut db_backup = File::open(path)?;
    let mut ds = DataStore::new();

    read_string_map(&mut db_backup, &mut ds.string_db)?;
    read_list_map(&mut db_backup, &mut ds.list_db)?;
    read_set_map(&mut db_backup, &mut ds.set_db)?;
    Ok(ds)
}
