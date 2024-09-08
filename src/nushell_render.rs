use gitql_core::object::GitQLObject;
use gitql_core::object::Row;
// use gix::config::key;
use nu_protocol::{Record, Value};
// use gix::objs::tag;

// enum PaginationInput {
//     NextPage,
//     PreviousPage,
//     Quit,
// }

pub fn render_objects(
    groups: &mut GitQLObject,
    hidden_selections: &[String],
    pagination: bool,
    page_size: usize,
) -> Value {
    if groups.len() > 1 {
        groups.flat()
    }
    // eprintln!("a");

    if groups.is_empty() || groups.groups[0].is_empty() {
        // eprintln!("a.1");

        return Value::test_string("No data to display".to_string());
    }

    let gql_group = groups.groups.first().unwrap();
    let gql_group_len = gql_group.len();
    // eprintln!("b");

    // Setup table headers
    // let header_color = comfy_table::Color::Green;
    let mut table_headers = vec![];
    for key in &groups.titles {
        // table_headers.push(comfy_table::Cell::new(key).fg(header_color));
        table_headers.push(key);
    }
    // eprintln!("c");

    // Print all data without pagination
    if !pagination || page_size >= gql_group_len {
        // eprintln!("d");

        print_group_as_table(
            &groups.titles,
            table_headers,
            &gql_group.rows,
            hidden_selections.len(),
        )
        // return;
    } else {
        // eprintln!("e");

        return Value::test_nothing();
    }

    // Setup the pagination mode
    // let number_of_pages = (gql_group_len as f64 / page_size as f64).ceil() as usize;
    // let mut current_page = 1;

    // loop {
    //     let start_index = (current_page - 1) * page_size;
    //     let end_index = (start_index + page_size).min(gql_group_len);

    //     let current_page_groups = &gql_group.rows[start_index..end_index];
    //     println!("Page {}/{}", current_page, number_of_pages);
    //     print_group_as_table(
    //         &groups.titles,
    //         table_headers.clone(),
    //         current_page_groups,
    //         hidden_selections.len(),
    //     )

    // let pagination_input = handle_pagination_input(current_page, number_of_pages);
    // match pagination_input {
    //     PaginationInput::NextPage => current_page += 1,
    //     PaginationInput::PreviousPage => current_page -= 1,
    //     PaginationInput::Quit => break,
    // }
    // }
}

fn print_group_as_table(
    titles: &[String],
    // table_headers: Vec<comfy_table::Cell>,
    table_headers: Vec<&String>,
    rows: &[Row],
    hidden_selection_count: usize,
) -> Value {
    // eprintln!("{table_headers:#?}");
    // eprintln!("{titles:#?}");

    // let mut table = comfy_table::Table::new();
    // let mut table = vec![];
    // let mut table_of_values = vec![];

    // // Setup table style
    // table.load_preset(comfy_table::presets::UTF8_FULL);
    // table.apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS);
    // table.set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

    // table.set_header(table_headers);

    // let titles_len = titles.len();
    let mut table_row_val: Vec<Value> = vec![];
    // Add rows to the table
    for row in rows {
        // let mut table_row: Vec<comfy_table::Cell> = vec![];
        // let mut table_row: Vec<String> = vec![];
        // let mut table_row_val: Vec<Value> = vec![];
        // for index in 0..titles_len {
        //     let value = row.values.get(index + hidden_selection_count).unwrap();
        //     // table_row.push(comfy_table::Cell::new(value.to_string()));
        //     table_row.push(value.to_string());
        // }

        let mut rec = Record::new();
        for (column_name, column_value) in titles.iter().zip(row.values.iter()) {
            match column_value {
                gitql_core::value::Value::Integer(i) => {
                    rec.insert(column_name, Value::test_int(*i));
                }
                gitql_core::value::Value::Float(f) => {
                    rec.insert(column_name, Value::test_float(*f));
                }
                gitql_core::value::Value::Text(t) => {
                    rec.insert(column_name, Value::test_string(t.to_string()));
                }
                gitql_core::value::Value::Boolean(b) => {
                    rec.insert(column_name, Value::test_bool(*b));
                }
                gitql_core::value::Value::DateTime(dt) => {
                    rec.insert(column_name, Value::test_string(dt.to_string()));
                }
                gitql_core::value::Value::Date(dt) => {
                    rec.insert(column_name, Value::test_string(dt.to_string()));
                }
                gitql_core::value::Value::Time(t) => {
                    rec.insert(column_name, Value::test_string(t.to_string()));
                }
                // gitql_core::value::Value::Array(a, b) => {
                //     rec.insert(
                //         column_name,
                //         Value::test_list(
                //             b.iter()
                //                 .map(|v| Value::test_string(v.to_string()))
                //                 .collect(),
                //         ),
                //     );
                // }
                gitql_core::value::Value::Array(a, b) => {
                    rec.insert(
                        column_name,
                        match a {
                            gitql_core::types::DataType::Text => Value::test_list(
                                b.iter()
                                    .map(|v| Value::test_string(v.to_string()))
                                    .collect(),
                            ),
                            gitql_core::types::DataType::Integer => Value::test_list(
                                b.iter().map(|v| Value::test_int(v.as_int())).collect(),
                            ),
                            gitql_core::types::DataType::Float => Value::test_list(
                                b.iter().map(|v| Value::test_float(v.as_float())).collect(),
                            ),
                            gitql_core::types::DataType::Boolean => Value::test_list(
                                b.iter().map(|v| Value::test_bool(v.as_bool())).collect(),
                            ),
                            gitql_core::types::DataType::Date => Value::test_list(
                                b.iter()
                                    .map(|v| Value::test_string(v.to_string()))
                                    .collect(),
                            ),
                            gitql_core::types::DataType::Time => Value::test_list(
                                b.iter()
                                    .map(|v| Value::test_string(v.to_string()))
                                    .collect(),
                            ),
                            gitql_core::types::DataType::DateTime => Value::test_list(
                                b.iter()
                                    .map(|v| Value::test_string(v.to_string()))
                                    .collect(),
                            ),
                            gitql_core::types::DataType::Array(_a) => Value::test_list(
                                b.iter()
                                    .map(|v| Value::test_string(v.to_string()))
                                    .collect(),
                            ),
                            gitql_core::types::DataType::Range(_r) => Value::test_list(
                                b.iter()
                                    .map(|v| Value::test_string(v.to_string()))
                                    .collect(),
                            ),
                            gitql_core::types::DataType::Variant(_vv) => Value::test_list(
                                b.iter()
                                    .map(|v| Value::test_string(v.to_string()))
                                    .collect(),
                            ),
                            gitql_core::types::DataType::Optional(_o) => Value::test_list(
                                b.iter()
                                    .map(|v| Value::test_string(v.to_string()))
                                    .collect(),
                            ),
                            gitql_core::types::DataType::Varargs(_v) => Value::test_list(
                                b.iter()
                                    .map(|v| Value::test_string(v.to_string()))
                                    .collect(),
                            ),
                            gitql_core::types::DataType::Dynamic(_d) => Value::test_list(
                                b.iter()
                                    .map(|v| Value::test_string(v.to_string()))
                                    .collect(),
                            ),
                            gitql_core::types::DataType::Undefined => todo!(),
                            gitql_core::types::DataType::Any => todo!(),
                            gitql_core::types::DataType::Null => todo!(),
                        }, // Value::test_list(
                           //     b.iter()
                           //         .map(|v| Value::test_string(v.to_string()))
                           //         .collect(),
                           // ),
                    );
                }

                gitql_core::value::Value::Range(a, b, c) => todo!(),
                gitql_core::value::Value::Null => todo!(),
            }
            // rec.insert(column_name, Value::test_string(column_value));
        }
        table_row_val.push(Value::test_record(rec));

        // let rec = table_headers
        //     .iter()
        //     .zip(table_row.iter())
        //     .map(|(a, b)| (a.to_owned(), b.to_owned()))
        //     .collect::<Vec<_>>();
        // eprintln!("{rec:#?}");
        // rec.iter().for_each(|(k, v)| {
        //     let key = *k;
        //     let val = v;
        //     table_row_val.push(Value::test_record(record!(
        //         key => Value::test_string(val)
        //     )));
        // });

        // table.add_row(table_row);
        // table.push(table_row);
        // table_of_values.push(Value::test_list(table_row_val))
    }

    // Print table
    // eprintln!("{table:#?}");
    // Value::test_list(table_of_values)
    Value::test_list(table_row_val)
}

// fn handle_pagination_input(current_page: usize, number_of_pages: usize) -> PaginationInput {
//     loop {
//         if current_page < 2 {
//             println!("Enter 'n' for next page, or 'q' to quit:");
//         } else if current_page == number_of_pages {
//             println!("'p' for previous page, or 'q' to quit:");
//         } else {
//             println!("Enter 'n' for next page, 'p' for previous page, or 'q' to quit:");
//         }

//         std::io::Write::flush(&mut std::io::stdout()).expect("flush failed!");

//         let mut line = String::new();
//         std::io::stdin()
//             .read_line(&mut line)
//             .expect("Failed to read input");

//         let input = line.trim();
//         if input == "q" || input == "n" || input == "p" {
//             match input {
//                 "n" => {
//                     if current_page < number_of_pages {
//                         return PaginationInput::NextPage;
//                     } else {
//                         println!("Already on the last page");
//                         continue;
//                     }
//                 }
//                 "p" => {
//                     if current_page > 1 {
//                         return PaginationInput::PreviousPage;
//                     } else {
//                         println!("Already on the first page");
//                         continue;
//                     }
//                 }
//                 "q" => return PaginationInput::Quit,
//                 _ => unreachable!(),
//             }
//         }

//         println!("Invalid input");
//     }
// }
