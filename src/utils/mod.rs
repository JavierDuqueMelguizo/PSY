use std::{any::Any, fmt::{Debug, Display}, path::Path};

use crate::{log_writeln};

pub mod logging;

/// Generate UUID_v4
pub fn generate_uuid_v4() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// CREATE AND LOAD DATA FROM FILES
pub async fn save_data(path: impl AsRef<Path>, data: &impl AsRef<[u8]>) -> Result<(), Box<dyn std::error::Error>>{
    tokio::fs::write(path, data).await?;
    Ok(())
}

pub async fn load_data(path: impl AsRef<Path> + Display + Clone) -> Option<Vec<u8>>{
    match tokio::fs::read(path.clone()).await{
        Ok(value) => Some(value),
        Err(e) => {
            eprintln!("Error leyendo fichero '{}'. Error: {}", path, e);
            None
        }
    }
}

/// DEBUG PODValues 
/// Este metodo solo existe para fines de Debug
pub fn print_pod(prop : &pipewire::spa::pod::PodProp){

    let padding = 4;
    let spaces = " ".repeat(padding);

    log_writeln!("--------------------------------");
    log_writeln!("{}Key: {:?}", spaces, prop.key());
    print_pod_value(prop.value(), Some(padding));
    log_writeln!("--------------------------------");
    
}
fn print_pod_value(value : &pipewire::spa::pod::Pod, padding : Option<usize>){

    let padding = padding.unwrap_or(0);
    let spaces = " ".repeat(padding);
    if value.is_object() {
        log_writeln!("{}BEGIN Object:", spaces);

        let object = value.as_object().expect(&format!("NO se ha podido extraer objeto"));
        log_writeln!("{}Object type: {:?}, Id: {:?}, Type Id: {:?} ", spaces, object.type_(), object.id(), object.type_id());
        
        for prop in object.props(){
            log_writeln!("{}Prop type: {:?} ", spaces, prop.type_id());
            print_pod(prop);
        }
      
        log_writeln!("{}END Object", spaces);
    } 
    else if value.is_struct(){
        log_writeln!("{}BEGIN Struct:", spaces);

        let structure = value.as_struct().expect(&format!("NO se ha podido extraer struct"));
        
        for (index,field) in structure.fields().into_iter().enumerate(){
            log_writeln!("{}Field[{}], Type: {:?}", spaces, index, field.type_id());
            print_pod_value(
                field,
                Some(padding + 4)
            );
        }
        log_writeln!("{}END Struct", spaces);
    }
    else if value.is_array(){
        log_writeln!("{}BEGIN Array", spaces);
        
        print_pod_array(value, Some(padding + 4));
    
        log_writeln!("{}END Array", spaces);
    }
    else if value.is_choice(){
        log_writeln!("{}BEGIN Choice", spaces);

        print_pod_choice(value, Some(padding + 4));

        log_writeln!("{}END Choice", spaces);
    }
    else if value.is_sequence(){
        log_writeln!("{}BEGIN Sequence", spaces);
        
        value.as_object().expect(&format!("NO se ha podido extraer secuencia"))
            .props().into_iter().for_each(
                |object| { print_pod(object);}
            );
            
        log_writeln!("{}END Sequence", spaces);
    }
    else if value.is_bitmap() || value.is_bytes(){
        let type_value = if value.is_bitmap() { "BITMAP" } else { "BYTES" };
        log_writeln!("{}BEGIN {}", spaces, type_value);
        
        let bytes = value.as_bytes();
        log_writeln!("{}Size: {} bytes", spaces, bytes.len());
        log_writeln!("{}Hex: {}", spaces, bytes.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" "));

        log_writeln!("{}END {}", spaces, type_value);
    }
    // TIPOS ESCALARES
    else if value.is_bool(){
        log_writeln!("{}Value (BOOL): {:?}", spaces, value.get_bool());
    }
     else if value.is_id(){
        log_writeln!("{}Value (ID): {:?}", spaces, value.get_id());
    }
    else if value.is_int(){
        log_writeln!("{}Value (INT): {:?}", spaces, value.get_int());
    }
    else if value.is_long(){
        log_writeln!("{}Value (LONG): {:?}", spaces, value.get_long());
    }
    else if value.is_float(){
        log_writeln!("{}Value (FLOAT): {:?}", spaces, value.get_float());
    }
    else if value.is_double(){
        log_writeln!("{}Value (DOUBLE): {:?}", spaces, value.get_double());
    }
    else if value.is_string(){
        log_writeln!("{}Value (STRING): {:?}", spaces, value.get_string_raw());
    }
    else if value.is_fd(){
        log_writeln!("{}Value (FD): {:?}", spaces, value.get_fd());
    }
    else if value.is_pointer(){
        log_writeln!("{}Value (POINTER): {:?}", spaces, value.get_pointer());
    }
    else if value.is_rectangle(){
        log_writeln!("{}Value (RECTANGLE): {:?}", spaces, value.get_rectangle());
    }
    else if value.is_fraction(){
        log_writeln!("{}Value (FRACTION): {:?}", spaces, value.get_fraction());
    }
    else if value.is_none(){
        log_writeln!("{}Value (NONE): {:?}", spaces, value.as_bytes());
    }
    else{
        log_writeln!(
            "{}UNKNOWN POD TYPE {:?}. Value TypeId: {:?}, Bytes: {:?}",
            spaces,
            value.type_(),
            value.type_id(),
            value.as_bytes()
        );
    }
}

fn print_pod_choice(value : &pipewire::spa::pod::Pod, padding : Option<usize>){

    use pipewire::spa::pod::deserialize::{PodDeserializer};
    use pipewire::spa::pod::{ChoiceValue, Value};
    use pipewire::spa::pod::deserialize::DeserializeError;

    let padding = padding.unwrap_or(0);
    let spaces = " ".repeat(padding);

    log_writeln!("{}Type: {:?}", spaces, value.type_());
    log_writeln!("{}TypeId: {:?}", spaces, value.type_id());

    let deserializer: Result<(&[u8], Value), DeserializeError<&[u8]>> = PodDeserializer::deserialize_from(value.as_bytes());
    if let Ok(choice) = deserializer{
        if let Value::Choice(choice_enum) = choice.1{
            log_writeln!("{}Choice Value: {:?}", spaces, choice_enum);
            match choice_enum {
                ChoiceValue::Bool(value) => print_pod_choice_value(value, Some(padding + 4)),
                ChoiceValue::Int(value) => print_pod_choice_value(value, Some(padding + 4)),
                ChoiceValue::Long(value) => print_pod_choice_value(value, Some(padding + 4)),
                ChoiceValue::Float(value) => print_pod_choice_value(value, Some(padding + 4)),
                ChoiceValue::Double(value) => print_pod_choice_value(value, Some(padding + 4)),
                ChoiceValue::Id(value) => print_pod_choice_value(value, Some(padding + 4)),
                ChoiceValue::Rectangle(value) => print_pod_choice_value(value, Some(padding + 4)),
                ChoiceValue::Fraction(value) => print_pod_choice_value(value, Some(padding + 4)),
                ChoiceValue::Fd(value) => print_pod_choice_value(value, Some(padding + 4)),
            }
        }
        else{
            log_writeln!("{}Error: El valor deserializado no es un Choice. Valor: {:?}", spaces, choice.1);
        }
    }
    else{
        log_writeln!("{}Error deserializando Choice: {:?}", spaces, deserializer.err());
        log_writeln!("{}Bytes: {:?}", spaces, value.as_bytes());
    }
    
}

fn print_pod_choice_value<T : pipewire::spa::pod::CanonicalFixedSizedPod + Debug>(value : pipewire::spa::utils::Choice<T>,padding : Option<usize>){

    use pipewire::spa::utils::ChoiceEnum;

    let padding = padding.unwrap_or(0);
    let spaces = " ".repeat(padding);

    log_writeln!("{}Choice Flags: {:?}", spaces, value.0);
    match value.1 {
        ChoiceEnum::None(value) => {log_writeln!("{}None: {:?}", spaces, value);},
        ChoiceEnum::Range { default, min, max } => {log_writeln!("{}Range: default: {:?}, min: {:?}, max: {:?}", spaces, default, min, max);},
        ChoiceEnum::Step { default, min, max, step } => {log_writeln!("{}Step: default: {:?}, min: {:?}, max: {:?}, step: {:?}", spaces, default, min, max, step);},
        ChoiceEnum::Enum { default, alternatives } => {log_writeln!("{}Enum: default: {:?}, alternatives: {:?}", spaces, default, alternatives);},
        ChoiceEnum::Flags { default, flags } => {log_writeln!("{}Flags: default: {:?}, flags: {:?}", spaces, default, flags);},
    }
}

fn print_pod_array(value : &pipewire::spa::pod::Pod, padding : Option<usize>){

    use pipewire::spa::pod::deserialize::{PodDeserializer as PwPodDeserializer};
    use pipewire::spa::pod::{Value as PwValue};
    use pipewire::spa::pod::deserialize::DeserializeError;

    let padding = padding.unwrap_or(0);
    let spaces = " ".repeat(padding);

    log_writeln!("{}Type: {:?}", spaces, value.type_());
    log_writeln!("{}TypeId: {:?}", spaces, value.type_id());

    let deserializer: Result<(&[u8], PwValue), DeserializeError<&[u8]>> = PwPodDeserializer::deserialize_from(value.as_bytes());
    if let Ok(array) = deserializer{
        if let PwValue::ValueArray(array_value) = array.1{
            log_writeln!("{}Array Value: {:?}", spaces, array_value);
            //print_pod_array_value(array_value, padding);
        }
        else{
            log_writeln!("{}Error: El valor deserializado no es un Array. Valor: {:?}", spaces, array.1);
        }
    }
    else{
        log_writeln!("{}Error deserializando Array: {:?}", spaces, deserializer.err());
        log_writeln!("{}Bytes: {:?}", spaces, value.as_bytes());
    }
}