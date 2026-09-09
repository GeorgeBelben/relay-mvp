use std::time::Duration;

fn main() {
    let mut gilrs = gilrs::Gilrs::new().unwrap();
    loop {
        while let Some(event) = gilrs.next_event() {
            match event.event {
                gilrs::EventType::AxisChanged(..) => {} // too noisy to print every tick
                other => println!("{other:?}"),
            }
        }
        std::thread::sleep(Duration::from_millis(16)); // ~60 times a second, like a game loop would poll
    }
}
