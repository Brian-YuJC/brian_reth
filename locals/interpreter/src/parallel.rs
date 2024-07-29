use core::str;
use std::{fs::File, io::Write, sync::RwLock, thread};
use revm_primitives::HashMap;
use once_cell::sync::Lazy;
use std::sync::{mpsc, Mutex};
use crate::OpCode;

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
}

pub struct Batch {
    pub block_msg_vec: Vec<BlockMsg>,
    pub write_path: Option<usize>,
}

pub static mut WRITE_PATH_VEC: Vec<Mutex<String>> = Vec::new();

pub fn start_channel(split: u64) -> thread::JoinHandle<()> {

    let mut block_msg: BlockMsg = BlockMsg{
        block_num: 0,
        op_time_map: HashMap::new(),
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
        }
    });
    log_handle
}

//Brian Add
static mut COUNT: RwLock<u64> = RwLock::<u64>::new(0); //已输出的block数量

//Brian Modify
pub fn print_records(thread_id: u64) -> thread::JoinHandle<()>{
    let print_handler: thread::JoinHandle<()> = thread::spawn(move || {
        loop {
            //eprintln!("Thread {} receiving", thread_id);
            let print_message = {
                let receiver = PRINT_CHANNEL.1.lock().unwrap();
                receiver.recv()
            };
            match print_message {
                Ok(message) => {
                    if message.write_path != None { //判断BlockMsg是否为空
                        let f_lock_res = unsafe { WRITE_PATH_VEC[message.write_path.unwrap()].lock()};
                        match f_lock_res{
                            Ok(f_gard) => {
                                let mut f = File::create(f_gard.clone()).unwrap();
                                for block_msg in message.block_msg_vec {
                                    f.write(format!("BlockNumber {}\n", block_msg.block_num).as_bytes()).unwrap();
                                    for (k, v) in block_msg.op_time_map { //Output
                                        f.write(format!("{}", k).as_bytes()).unwrap();
                                        for op_time in v {
                                            f.write(format!(" {}", op_time).as_bytes()).unwrap();
                                        }
                                        f.write(String::from("\n").as_bytes()).unwrap();
                                    }
                                    f.flush().unwrap();
                                    unsafe { //标记提交打印次数
                                        *COUNT.write().unwrap() += 1;
                                        eprintln!("Thread {}. Done {}", thread_id, *COUNT.read().unwrap());
                                    }
                                }
                            }
                            Err(_) => { 
                                
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