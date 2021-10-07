///`modes` is a module containing tools to live acquire frames and spectral images.
use crate::packetlib::{Packet, PacketEELS as Pack};
use crate::auxiliar::Settings;
use crate::tdclib::{TdcControl, PeriodicTdcRef};
use std::time::Instant;
use std::net::TcpStream;
use std::io::{Read, Write};
use std::sync::mpsc;
use std::thread;

const CAM_DESIGN: (usize, usize) = Pack::chip_array();
const BUFFER_SIZE: usize = 16384 * 4;

pub fn build_spectrum_thread<T: 'static + TdcControl + Send>(mut pack_sock: TcpStream, mut ns_sock: TcpStream, my_settings: Settings, mut frame_tdc: PeriodicTdcRef, mut ref_tdc: T) {
    
    let (tx, rx) = mpsc::channel();
    let start = Instant::now();
    let mut last_ci = 0usize;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    let mut data_array:Vec<u8> = vec![0; ((CAM_DESIGN.1-1)*!my_settings.bin as usize + 1)*my_settings.bytedepth*CAM_DESIGN.0];
    data_array.push(10);

    thread::spawn(move || {
        loop {
            if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                if size>0 {
                    let new_data = &buffer_pack_data[0..size];
                        if build_data(new_data, &mut data_array, &mut last_ci, &my_settings, &mut frame_tdc, &mut ref_tdc) {
                            let msg = create_header(&my_settings, &frame_tdc);
                            tx.send((data_array.clone(), msg)).expect("could not send data in the thread channel.");
                            if my_settings.cumul == false {
                                data_array = vec![0; ((CAM_DESIGN.1-1)*!my_settings.bin as usize + 1)*my_settings.bytedepth*CAM_DESIGN.0];
                                data_array.push(10);
                            };
                            if frame_tdc.counter() % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter());}
                        }
                }
            }
        }
    });

    loop {
        if let Ok((result, msg)) = rx.recv() {
            if let Err(_) = ns_sock.write(&msg) {println!("Client disconnected on data."); break;}
            if let Err(_) = ns_sock.write(&result) {println!("Client disconnected on data."); break;}
        } else {break;}
    }
}



///Reads timepix3 socket and writes in the output socket a header and a full frame (binned or not). A periodic tdc is mandatory in order to define frame time.
pub fn build_spectrum<T: TdcControl>(mut pack_sock: TcpStream, mut vec_ns_sock: Vec<TcpStream>, my_settings: Settings, mut frame_tdc: PeriodicTdcRef, mut ref_tdc: T) {

    let start = Instant::now();
    let mut last_ci = 0usize;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    let mut data_array:Vec<u8> = vec![0; ((CAM_DESIGN.1-1)*!my_settings.bin as usize + 1)*my_settings.bytedepth*CAM_DESIGN.0];
    data_array.push(10);

    while let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
        if size == 0 {println!("Timepix3 sent zero bytes."); break;}
        let new_data = &buffer_pack_data[0..size];
        if build_data(new_data, &mut data_array, &mut last_ci, &my_settings, &mut frame_tdc, &mut ref_tdc) {
            let msg = create_header(&my_settings, &frame_tdc);
            if let Err(_) = vec_ns_sock[0].write(&msg) {println!("Client disconnected on header."); break;}
            if let Err(_) = vec_ns_sock[0].write(&data_array) {println!("Client disconnected on data."); break;}
            if my_settings.cumul == false {
                data_array = vec![0; ((CAM_DESIGN.1-1)*!my_settings.bin as usize + 1)*my_settings.bytedepth*CAM_DESIGN.0];
                data_array.push(10);
            };
            if frame_tdc.counter() % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter());}
        }
    }
}

fn build_data<T: TdcControl>(data: &[u8], final_data: &mut [u8], last_ci: &mut usize, settings: &Settings, frame_tdc: &mut PeriodicTdcRef, ref_tdc: &mut T) -> bool {

    let mut packet_chunks = data.chunks_exact(8);
    let mut has = false;
    
    while let Some(x) = packet_chunks.next() {
        match x {
            &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci as usize,
            _ => {
                let packet = Pack { chip_index: *last_ci, data: x};
                
                match packet.id() {
                    11 if ref_tdc.period().is_none() => {
                        let array_pos = match settings.bin {
                            false => packet.x() + CAM_DESIGN.0*packet.y(),
                            true => packet.x()
                        };
                        append_to_array(final_data, array_pos, settings.bytedepth);
                    },
                    11 if ref_tdc.period().is_some() => {
                        if let Some(_backtdc) = tr_check_if_in(packet.electron_time(), ref_tdc.time(), ref_tdc.period().unwrap(), settings) {
                            let array_pos = match settings.bin {
                                false => packet.x() + CAM_DESIGN.0*packet.y(),
                                true => packet.x()
                            };
                            append_to_array(final_data, array_pos, settings.bytedepth);
                        }
                    },
                    6 if packet.tdc_type() == frame_tdc.id() => {
                        frame_tdc.upt(packet.tdc_time(), packet.tdc_counter());
                        has = true;
                    },
                    6 if packet.tdc_type() == ref_tdc.id() => {
                        ref_tdc.upt(packet.tdc_time_norm(), packet.tdc_counter());
                        if ref_tdc.period().is_none() {
                            append_to_array(final_data, CAM_DESIGN.0-1, settings.bytedepth);
                        }   
                    },
                    _ => {},
                };
            },
        };
    };
    has
}

fn tr_check_if_in(ele_time: f64, tdc: f64, period: f64, settings: &Settings) -> Option<usize> {
    let mut eff_tdc = tdc;
    let mut counter = 0;
    while ele_time < eff_tdc {
        counter+=1;
        eff_tdc = eff_tdc - period;
    }
    
    if ele_time > eff_tdc + settings.time_delay && ele_time < eff_tdc + settings.time_delay + settings.time_width {
        Some(counter)
    } else {
        None
    }
}

fn append_to_array(data: &mut [u8], index:usize, bytedepth: usize) {
    let index = index * bytedepth;
    match bytedepth {
        4 => {
            data[index+3] = data[index+3].wrapping_add(1);
            if data[index+3]==0 {
                data[index+2] = data[index+2].wrapping_add(1);
                if data[index+2]==0 {
                    data[index+1] = data[index+1].wrapping_add(1);
                    if data[index+1]==0 {
                        data[index] = data[index].wrapping_add(1);
                    };
                };
            };
        },
        2 => {
            data[index+1] = data[index+1].wrapping_add(1);
            if data[index+1]==0 {
                data[index] = data[index].wrapping_add(1);
            }
        },
        1 => {
            data[index] = data[index].wrapping_add(1);
        },
        _ => {panic!("Bytedepth must be 1 | 2 | 4.");},
    }
}

fn create_header<T: TdcControl>(set: &Settings, tdc: &T) -> Vec<u8> {
    let mut msg: String = String::from("{\"timeAtFrame\":");
    msg.push_str(&(tdc.time().to_string()));
    msg.push_str(",\"frameNumber\":");
    msg.push_str(&(tdc.counter().to_string()));
    msg.push_str(",\"measurementID:\"Null\",\"dataSize\":");
    match set.bin {
        true => { msg.push_str(&((set.bytedepth*CAM_DESIGN.0).to_string()))},
        false => { msg.push_str(&((set.bytedepth*CAM_DESIGN.0*CAM_DESIGN.1).to_string()))},
    }
    msg.push_str(",\"bitDepth\":");
    msg.push_str(&((set.bytedepth<<3).to_string()));
    msg.push_str(",\"width\":");
    msg.push_str(&(CAM_DESIGN.0.to_string()));
    msg.push_str(",\"height\":");
    match set.bin {
        true=>{msg.push_str(&(1.to_string()))},
        false=>{msg.push_str(&(CAM_DESIGN.1.to_string()))},
    }
    msg.push_str("}\n");

    let s: Vec<u8> = msg.into_bytes();
    s
}
