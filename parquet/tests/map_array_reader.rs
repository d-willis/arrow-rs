use arrow::array::{self, MapBuilder, PrimitiveBuilder, StringBuilder};
use arrow::datatypes::{DataType, Field, Int32Type, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::{ArrowReader, ArrowWriter, ParquetFileArrowReader};
use parquet::file::properties::WriterProperties;
use std::fs;
use std::path::Path;
use std::sync::Arc;

// Struct to delete file when test finishes, whether successfully or not
struct FileRemover {
    filename: String,
}

impl Drop for FileRemover {
    fn drop(&mut self) {
        if Path::new(&self.filename).exists() {
            fs::remove_file(&self.filename).expect("Removing file");
        }
    }
}

#[test]
// This test writes a parquet file with the following data:
// +--------------------------------------------------------+
// |map                                                     |
// +--------------------------------------------------------+
// |null                                                    |
// |null                                                    |
// |{three -> 3, four -> 4, five -> 5, six -> 6, seven -> 7}|
// +--------------------------------------------------------+
//
// It then attempts to read the data back and checks that the third record
// contains the expected values.
fn read_map_array_column() {
    // Make sure generated parquet file is removed whether test passes or not
    let _file_remover = FileRemover {
        filename: "read_map_column_with_leading_nulls.parquet".to_string(),
    };

    // Schema for single map of string to int32
    let path = Path::new("read_map_column_with_leading_nulls.parquet");
    let schema = Schema::new(vec![Field::new(
        "map",
        DataType::Map(
            Box::new(Field::new(
                "entries",
                DataType::Struct(vec![
                    Field::new("keys", DataType::Utf8, false),
                    Field::new("values", DataType::Int32, true),
                ]),
                false,
            )),
            false, // Map field not sorted
        ),
        true,
    )]);

    // Create builders for map
    let string_builder = StringBuilder::new(5);
    let ints_builder: PrimitiveBuilder<Int32Type> = PrimitiveBuilder::new(1);
    let mut map_builder = MapBuilder::new(None, string_builder, ints_builder);

    // Add two null records and one record with three entries
    map_builder.append(false).expect("adding null map entry");
    map_builder.append(false).expect("adding null map entry");
    map_builder.keys().append_value("three");
    map_builder.keys().append_value("four");
    map_builder.keys().append_value("five");
    map_builder.keys().append_value("six");
    map_builder.keys().append_value("seven");

    map_builder.values().append_value(3);
    map_builder.values().append_value(4);
    map_builder.values().append_value(5);
    map_builder.values().append_value(6);
    map_builder.values().append_value(7);
    map_builder.append(true).expect("adding map entry");

    // Create record batch
    let batch =
        RecordBatch::try_new(Arc::new(schema), vec![Arc::new(map_builder.finish())])
            .expect("create record batch");

    // Write record batch to file
    let props = WriterProperties::builder().build();
    let file = fs::File::create(&path).expect("create file");
    let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))
        .expect("creat file writer");
    writer.write(&batch).expect("writing file");
    writer.close().expect("close writer");

    // Read file
    let file = fs::File::open(&Path::new("read_map_column_with_leading_nulls.parquet"))
        .expect("open file");
    let mut arrow_reader =
        ParquetFileArrowReader::try_new(file).expect("Trying to read parquet file");
    let record_batch_reader = arrow_reader.get_record_reader(1024);
    for maybe_record_batch in record_batch_reader.expect("Getting batch iterator") {
        let record_batch = maybe_record_batch.expect("Getting current batch");
        let col = record_batch.column(0);
        let map_entry = array::as_map_array(col).value(2);
        let struct_col = array::as_struct_array(&map_entry);
        let key_col = array::as_string_array(struct_col.column(0)); // Key column
        assert_eq!(key_col.value(0), "three");
        assert_eq!(key_col.value(1), "four");
        assert_eq!(key_col.value(2), "five");
        assert_eq!(key_col.value(3), "six");
        assert_eq!(key_col.value(4), "seven");
    }
    println!("finished reading");
}
