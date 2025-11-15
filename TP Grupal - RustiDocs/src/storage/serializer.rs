//! Funciones para serializar el datastore

// IMPORTS
use crate::storage::DataStore;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io;
use std::io::Write;

// FUNCIONES

/// Función auxiliar para escribir una cadena de caracteres en un archivo
fn write_string<V, W>(writer: &mut W, str: V) -> io::Result<()>
where
    V: AsRef<str>,
    W: Write,
{
    let str = str.as_ref();
    let len = str.len();
    writer.write_all(&len.to_be_bytes())?;
    writer.write_all(str.as_bytes())?;
    Ok(())
}

/// Función auxiliar para iterar sobre un HashMap y serializar sus
/// componentes "iterables" en un archivo
fn iterate_and_write<T, K, V, VI>(db: T, dest: &mut File) -> io::Result<()>
where
    T: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: IntoIterator<Item = VI>,
    VI: AsRef<str>,
{
    for (key, value) in db {
        write_string(dest, key)?;
        let value = value.into_iter().collect::<Vec<_>>();
        dest.write_all(&value.len().to_be_bytes())?;
        for item in value {
            write_string(dest, item)?;
        }
    }
    Ok(())
}

/// Serializa un HashMap de Vectores de Strings a un archivo
fn serialize_vec_nested_hm(db: &HashMap<String, Vec<String>>, dest: &mut File) -> io::Result<()> {
    let list_db_len = db.len();
    dest.write_all(&list_db_len.to_be_bytes())?;
    iterate_and_write(db, dest)?;
    Ok(())
}

/// Serializa un HashMap de HashSets de Strings a un archivo
fn serialize_set_nested_hm(
    db: &HashMap<String, HashSet<String>>,
    dest: &mut File,
) -> io::Result<()> {
    let set_db_len = db.len();
    dest.write_all(&set_db_len.to_be_bytes())?;
    iterate_and_write(db, dest)?;
    Ok(())
}

/// Serializa un HashMap de Strings a un archivo
fn serialize_simple_hm<W: Write>(db: &HashMap<String, String>, dest: &mut W) -> io::Result<()> {
    let db_len = db.len();
    dest.write_all(&db_len.to_be_bytes())?;
    for (key, value) in db.iter() {
        write_string(dest, key)?;
        write_string(dest, value)?;
    }
    Ok(())
}

/// Itera sobre el datastore y serializa los datos en un archivo
/// a medida que lo recorre parra evitar guardar todo el archivo
/// en memoria al mismo tiempo.
pub fn serialize_ds(ds: &DataStore, dest: &mut File) -> Result<(), io::Error> {
    serialize_simple_hm(&ds.string_db, dest)?;
    serialize_vec_nested_hm(&ds.list_db, dest)?;
    serialize_set_nested_hm(&ds.set_db, dest)?;
    Ok(())
}
