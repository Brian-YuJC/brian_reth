use core::str;
use std::{fs::File, sync::mpsc::RecvError, thread};
use revm_primitives::{alloy_primitives::Sealable, HashMap};
use once_cell::sync::Lazy;
use std::sync::{mpsc, Mutex};
use crate::OpCode;
use lazy_static::lazy_static;


pub static OP_CHANNEL: Lazy<(mpsc::Sender<OpcodeMsg>, Mutex<mpsc::Receiver<OpcodeMsg>>)> = Lazy::new(|| { //Brian Add
    let (sender, receiver) = mpsc::channel();
    (sender, Mutex::new(receiver))
});

pub static PRINT_CHANNEL: Lazy<(mpsc::Sender<BlockMsg<'_>>, Mutex<mpsc::Receiver<BlockMsg<'_>>>)> = Lazy::new(|| { //Brian Add
    let (sender, receiver) = mpsc::channel();
    (sender, Mutex::new(receiver))
});

pub struct OpcodeMsg { // Brian Add
    pub op_idx: u8,
    pub run_time: u128,
    //pub writer_ptr: File,
}

pub struct BlockMsg<'a> { //Brian Add
    pub op_name_list: Vec<&'a str>,
    pub run_time_list: Vec<u128>,
    //pub write_ptr: File,
}

//pub static mut WRITE_FILE: &File = *File::create("./tmp").unwrap();
lazy_static! { //Brian Add
    pub static ref WRITE_PATH: String = String::new();
}


 
// 使用 lazy_static 来创建一个全局的 HashMap，并用 Mutex 封装
lazy_static! {
    static ref OP_COUNT_MAP: Mutex<HashMap<&'static str, u128>> = Mutex::new(HashMap::new());
}
lazy_static! {
    static ref OP_TIME_MAP: Mutex<HashMap<&'static str, u128>> = Mutex::new(HashMap::new());
}

// 创建一个全局的 mpsc::channel，并用 Mutex 封装接收端
static CHANNEL: Lazy<(mpsc::Sender<(u8, u128, u128)>, Mutex<mpsc::Receiver<(u8, u128, u128)>>)> = Lazy::new(|| {
    let (sender, receiver) = mpsc::channel();
    (sender, Mutex::new(receiver))
});

pub fn start_channel() -> thread::JoinHandle<()> {

    let mut block_msg: BlockMsg<'_> = BlockMsg{
        op_name_list: Vec::new(),
        run_time_list: Vec::new(),
    }; //Brian Add

    //Brian Modify
    let log_handle: thread::JoinHandle<()> = thread::spawn(|| { // 启动一个线程来处理日志
        loop {
            let log_message = { // 锁定接收端，并尝试接收消息
                let receiver = OP_CHANNEL.1.lock().unwrap();
                receiver.recv()
            };

            match log_message {
                Ok(message) => {
                    if message.op_idx == 0xCC { //0xCC还没对应的opcode，可以拿来做新Block标识，运行到0xCC表示开始运行新块
                        PRINT_CHANNEL.0.send(block_msg).unwrap(); //将旧blockmsg实例放入打印管道，等候被打印
                        block_msg = BlockMsg{ //新建新块的消息实例
                            op_name_list: Vec::new(),
                            run_time_list: Vec::new(),
                        };
                    }
                    else { //往blockMsg实例里填opcode
                        block_msg.op_name_list.push(OpCode::new(message.op_idx).unwrap().as_str());
                        block_msg.run_time_list.push(message.run_time);
                    }
                }
                Err(_) => { // 当发送端关闭时，退出循环
                    break
                }
            }
        }
    });

    // 启动一个线程来处理日志
    // let log_handle: thread::JoinHandle<()> = thread::spawn(|| {
    //     loop {
    //         // 锁定接收端，并尝试接收消息
    //         let log_message = {
    //             let receiver = CHANNEL.1.lock().unwrap();
    //             receiver.recv()
    //         };

    //         // 处理接收到的消息
    //         match log_message {
    //             Ok(message) => {
    //                 // 在这里写日志，例如，写入文件或打印到控制台
    //                 let input_op = message.0;
    //                 let input_op_count = message.1;
    //                 let input_op_time = message.2;
    //                 let op_code = OpCode::new(input_op).unwrap().as_str();

    //                 let mut op_count_map_temp = OP_COUNT_MAP.lock().unwrap();
    //                 let op_count = op_count_map_temp.entry(&op_code).or_insert(0);
    //                 *op_count += input_op_count;

    //                 let mut op_time_map_temp = OP_TIME_MAP.lock().unwrap();
    //                 let op_time = op_time_map_temp.entry(&op_code).or_insert(0);
    //                 *op_time += input_op_time;
    //                 }
    //             Err(_) => {
    //                 // 当发送端关闭时，退出循环
    //                 break;
    //             }
    //         }
    //     }
    // });
    log_handle
}

pub fn update_total_op_count_and_time(op_list: [u128; 256], run_time_list: [u128; 256]) {
    // let start = Instant::now();
    thread::spawn(move || {
        for op_idx in 0..256 {
            let op = op_idx as u8;
            let op_count = op_list[op_idx];
            if op_count > 0 {
                let op_run_time = run_time_list[op_idx];
                CHANNEL.0.send((op, op_count, op_run_time)).unwrap();
            }
        }
    });
    // let end = Instant::now();
    // let elapsed_ns = end.duration_since(start).as_nanos();
    // println!("Run time as nanos: {:?}", elapsed_ns);
}

//Brian Modify
pub fn print_records() -> thread::JoinHandle<()>{
    // for (result_op_code, result_op_count) in OP_COUNT_MAP.lock().unwrap().iter() {
    //     let result_op_code_str = *result_op_code;
    //     let result_op_count_str = *result_op_count;
    //     let result_op_total_run_time = *OP_TIME_MAP.lock().unwrap().get(result_op_code).unwrap();
    //     println!("Opcode name is: {:?}. Run time as nanos: {:?}. Total Count is: {:?}", result_op_code_str, result_op_total_run_time, result_op_count_str);
    // }
    let print_handler: thread::JoinHandle<()> = thread::spawn(|| {
        loop {
            let print_message = {
                let receiver = PRINT_CHANNEL.1.lock().unwrap();
                receiver.recv()
            };
            match print_message {
                Ok(message) => {
                    if message.op_name_list.len() > 0 { //判断BlockMsg是否为空
                        for opcode in message.op_name_list {
                        println!("{}", opcode)
                        }
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }
    });
    print_handler
}