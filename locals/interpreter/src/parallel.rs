use core::{borrow::Borrow, ptr::null, str};
use std::{fmt::format, fs::{File, OpenOptions}, io::Write, path, sync::{mpsc::RecvError, RwLock}, thread};
use revm_primitives::{alloy_primitives::Sealable, HashMap};
use once_cell::sync::Lazy;
use std::sync::{mpsc, Mutex};
use crate::{opcode::make_instruction_table, OpCode};
use lazy_static::lazy_static;

//Brian Add
pub static OP_CHANNEL: Lazy<(mpsc::Sender<OpcodeMsg>, Mutex<mpsc::Receiver<OpcodeMsg>>)> = Lazy::new(|| { 
    let (sender, receiver) = mpsc::channel();
    (sender, Mutex::new(receiver))
});

//Brian Add
pub static PRINT_CHANNEL: Lazy<(mpsc::Sender<Batch>, Mutex<mpsc::Receiver<Batch>>)> = Lazy::new(|| { 
    let (sender, receiver) = mpsc::channel();
    (sender, Mutex::new(receiver))
});

// Brian Add
pub struct OpcodeMsg { 
    pub op_idx: u8,
    pub run_time: u128,
    pub writer_path: Option<usize>,
}

//Brian Add
pub struct BlockMsg { 
    pub block_num: u128,
    pub op_time_map: HashMap<&'static str, Vec<u128>>,
    //pub write_path: Option<usize>, //may be change to vec index

    //pub op_name_list: Vec<&'a str>,
    //pub run_time_list: Vec<u128>,
}

pub struct Batch {
    pub block_msg_vec: Vec<BlockMsg>,
    pub write_path: Option<usize>,
}

//pub static mut WRITE_FILE: &File = *File::create("./tmp").unwrap();

// lazy_static! { //Brian Add
//     pub static ref WRITE_PATH: String = String::new();
// }
//Brian Add
//pub static mut WRITE_PATH: &String = &String::new();
// pub static mut WRITE_FILE: Lazy<File> = Lazy::new(|| {
//     //File::create_new("./output/0.txt").unwrap()
//     None::<File>.unwrap()
// });
// pub static mut WRITE_FILE: Lazy<(File, String)> = Lazy::new(|| {
//     (None::<File>.unwrap(), String::new())
// });
//pub static mut WRITE_PATH: &String = &String::new();
pub static mut WRITE_PATH_VEC: Vec<Mutex<String>> = Vec::new();
//pub static mut WRITE_PATH: Option<String> = None;
 
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

pub fn start_channel(split: u64) -> thread::JoinHandle<()> {

    // unsafe {
    //     let mut file = OpenOptions::new().append(true).open(WRITE_PATH).expect("Can not open file!");
    // }

    let mut block_msg: BlockMsg = BlockMsg{
        block_num: 0,
        op_time_map: HashMap::new(),
        //write_path: None::<&String>.unwrap(), //fix to WRITE_PATH_VEC.last().unwarp?
        //write_path: None,
        //op_name_list: Vec::new(),
        //run_time_list: Vec::new(),
    }; //Brian Add

    let mut batch = Batch{
        block_msg_vec: Vec::<BlockMsg>::new(),
        write_path: None,
    };
    
    let mut counter: u64 = 0;

    //Brian Modify
    let log_handle: thread::JoinHandle<()> = thread::spawn(move || { // 启动一个线程来处理日志
        loop {
            let log_message = { // 锁定接收端，并尝试接收消息
                let receiver = OP_CHANNEL.1.lock().unwrap();
                receiver.recv()
            };

            match log_message {
                Ok(message) => {
                    if message.op_idx == 0xCC {
                        if block_msg.block_num != 0 {
                            batch.block_msg_vec.push(block_msg);
                            counter += 1;
                        } else {
                            batch.write_path = message.writer_path;
                        }
                        if counter == split {
                            PRINT_CHANNEL.0.send(batch).unwrap();
                            //eprintln!("Send batch------------------------------------------------------------------------------");
                            batch = Batch{
                                block_msg_vec: Vec::<BlockMsg>::new(),
                                write_path: message.writer_path,
                            };
                            counter = 0;
                        }
                        block_msg = BlockMsg{
                            block_num: message.run_time,
                            op_time_map: HashMap::new(),
                        };
                    }
                    else {
                        let opcode_str = OpCode::new(message.op_idx).unwrap().as_str();
                        let value = block_msg.op_time_map.get_mut(opcode_str); //区分get_mut 和 get 
                        match value {
                            Some(vec) => {
                                vec.push(message.run_time);
                            }
                            None => {
                                let mut vec: Vec<u128> = Vec::new();
                                vec.push(message.run_time);
                                block_msg.op_time_map.insert(OpCode::new(message.op_idx).unwrap().as_str(), vec);
                            }
                        }
                    }
                }
                Err(_) => {
                    break;
                }
            }


            // match log_message {
            //     Ok(message) => {
            //         if message.op_idx == 0xCC { //0xCC还没对应的opcode，可以拿来做新Block标识，运行到0xCC表示开始运行新块
            //             // PRINT_CHANNEL.0.send(block_msg).unwrap(); //将旧blockmsg实例放入打印管道，等候被打印
            //             // block_msg = BlockMsg{ //新建新块的消息实例
            //             //     block_num: message.run_time,
            //             //     op_time_map: HashMap::new(),
            //             //     write_path: Some(message.writer_path.unwrap()),
            //             //     //op_name_list: Vec::new(),
            //             //     //run_time_list: Vec::new(),
            //             // };
            //             let write_path: Option<usize>;
            //             PRINT_CHANNEL.0.send(block_msg).unwrap();
            //             match message.writer_path {
            //                 Some(val) => {
            //                     write_path = Some(val);
            //                 }
            //                 None => {
            //                     write_path = None;
            //                 }
            //             }
            //             block_msg = BlockMsg{ //新建新块的消息实例
            //                 block_num: message.run_time,
            //                 op_time_map: HashMap::new(),
            //                 //write_path: write_path,
            //                 //op_name_list: Vec::new(),
            //                 //run_time_list: Vec::new(),
            //             };
            //         }
            //         else { //往blockMsg实例里填opcode
            //             let opcode_str = OpCode::new(message.op_idx).unwrap().as_str();
            //             let value = block_msg.op_time_map.get_mut(opcode_str); //区分get_mut 和 get 
            //             match value {
            //                 Some(vec) => {
            //                     vec.push(message.run_time);
            //                 }
            //                 None => {
            //                     let mut vec: Vec<u128> = Vec::new();
            //                     vec.push(message.run_time);
            //                     block_msg.op_time_map.insert(OpCode::new(message.op_idx).unwrap().as_str(), vec);
            //                 }
            //             }
            //             //block_msg.op_name_list.push(OpCode::new(message.op_idx).unwrap().as_str());
            //             //block_msg.run_time_list.push(message.run_time);
            //         }
            //     }
            //     Err(_) => { // 当发送端关闭时，退出循环
            //         break
            //     }
            //}
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

//Brian Add
static mut COUNT: RwLock<u64> = RwLock::<u64>::new(0); //已输出的block数量

//Brian Modify
pub fn print_records(thread_id: u64) -> thread::JoinHandle<()>{
    // for (result_op_code, result_op_count) in OP_COUNT_MAP.lock().unwrap().iter() {
    //     let result_op_code_str = *result_op_code;
    //     let result_op_count_str = *result_op_count;
    //     let result_op_total_run_time = *OP_TIME_MAP.lock().unwrap().get(result_op_code).unwrap();
    //     println!("Opcode name is: {:?}. Run time as nanos: {:?}. Total Count is: {:?}", result_op_code_str, result_op_total_run_time, result_op_count_str);
    // }
    let print_handler: thread::JoinHandle<()> = thread::spawn(move || {
        loop {
            //eprintln!("Thread {} receiving", thread_id);
            let print_message = {
                let receiver = PRINT_CHANNEL.1.lock().unwrap();
                receiver.recv()
            };
            match print_message {
                Ok(message) => {
                    if /*message.op_name_list.len() > 0*/ message.write_path != None { //判断BlockMsg是否为空
                        // let path_lock = unsafe { &WRITE_PATH_VEC[message.write_path.unwrap()] };
                        // let path_guard1 = path_lock.lock().unwrap();
                        // let path_guard2 = path_lock.lock().unwrap();
                        // let file = OpenOptions::new().append(true).open(*path_guard1);
                        // let mut f;
                        // match file {
                        //     Ok(obj) => {
                        //         f = obj;
                        //     }
                        //     Err(_) => {
                        //         //f = File::create_new(*path_guard2).unwrap();
                        //     }
                        // }

                        let f_lock_res = unsafe { WRITE_PATH_VEC[message.write_path.unwrap()].lock()};
                        //添加判断，如果有其他线程在当前blockMsg相同地址进行输出，则将当前blockMsg再次推入管道
                        match f_lock_res{
                            Ok(f_gard) => { //没有被占用，进行追加写入操作
                                //print!("{}", f_gard.clone());
                                //let mut f = OpenOptions::new().append(false).open(f_gard.clone()).unwrap();
                                let mut f = File::create(f_gard.clone()).unwrap();
                                //eprintln!("block_msg_vec len: {}", message.block_msg_vec.len());
                                for block_msg in message.block_msg_vec {
                                    f.write(format!("BlockNumber {}\n", block_msg.block_num).as_bytes()).unwrap();
                                    for (k, v) in block_msg.op_time_map { //Output
                                        //print!("{}", k);
                                        f.write(format!("{}", k).as_bytes()).unwrap();
                                        for op_time in v {
                                            //print!(" {}", op_time);
                                            f.write(format!(" {}", op_time).as_bytes()).unwrap();
                                        }
                                        //print!("\n");
                                        f.write(String::from("\n").as_bytes()).unwrap();
                                    }
                                    f.flush().unwrap();
                                    unsafe { //标记提交打印次数
                                        *COUNT.write().unwrap() += 1;
                                        eprintln!("Thread {}. Done {}", thread_id, *COUNT.read().unwrap());
                                    }
                                }
                            }
                            Err(_) => { //冲突，推入管道
                                
                            }
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

//Brian Add
pub fn wait(block_cnt: u64, split: u64) { //等待执行完毕
    unsafe {
        loop {
            if *COUNT.read().unwrap() == block_cnt - split { //已经执行完倒数第二个
                OP_CHANNEL.0.send(OpcodeMsg{op_idx: 0xCC, run_time: 0, writer_path: None}).unwrap(); //发个信号，将最后一个blockMsg放进管道
                loop {
                    if *COUNT.read().unwrap() == block_cnt {
                        thread::sleep(core::time::Duration::from_secs(3));
                        return;
                    }
                }
            }
            thread::sleep(core::time::Duration::from_secs(1));
        }
    } 
}