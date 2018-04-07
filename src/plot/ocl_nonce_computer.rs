extern crate ocl;
extern crate num_iter;

use ocl::{Platform, Device, Queue, Buffer, Kernel};
use ocl::builders::{BufferBuilder, ContextBuilder, ProgramBuilder};
use failure::Error;

use util::config::{DeviceConfig};
use util::constants::GEN_SIZE;
use std::sync::mpsc::Receiver;
use std::thread;
use std::sync::mpsc::SyncSender;
use plot::PlotResult;


pub struct OclNonceComputer {
    device_config: DeviceConfig,
    current_nonce: u64,
    buffer: Buffer<u8>,
    nonce_step_2: Kernel,
    nonce_step_3: Kernel,
}

impl OclNonceComputer {
    pub fn set_nonce(&mut self, nonce: u64) {
        self.current_nonce = nonce;
    }

    pub fn current_nonce(&self) -> u64 {
        self.current_nonce
    }

    pub fn new(device_config: DeviceConfig, address: u64) -> Result<Self, Error> {
        // set up OpenCL kernels
        let platform = Platform::list()[device_config.platform_id() as usize];
        let device = Device::list_all(platform.clone())?[device_config.device_id() as usize];

        let context = ContextBuilder::new()
            .platform(platform)
            .devices(device)
            .build()?;

        let program = ProgramBuilder::new()
            .src_file("kernel/shabal.cl")
            .src_file("kernel/util.cl")
            .src_file("kernel/nonce.cl")
            .build(&context)?;

        let queue = Queue::new(&context, device, None)?;

        let buffer: Buffer<u8> = BufferBuilder::new()
            .queue(queue.clone())
            .len(device_config.global_work_size() as usize * GEN_SIZE)
            .build()?;

        let nonce_step_2 = Kernel::new("nonce_step2", &program)?
            .queue(queue.clone())
            .gws(device_config.global_work_size())
            .lws(device_config.local_work_size())
            .arg_buf(&buffer)
            .arg_scl_named("p_size", Some(device_config.global_work_size()))
            .arg_scl_named::<u64>("p_startNonce", None)
            .arg_scl_named::<u64>("p_address", Some(address));

        let nonce_step_3 = Kernel::new("nonce_step3", &program)?
            .queue(queue.clone())
            .gws(device_config.global_work_size())
            .lws(device_config.local_work_size())
            .arg_buf(&buffer)
            .arg_scl_named("p_size", Some(device_config.global_work_size()));

        Ok(Self { device_config, current_nonce: 0u64, buffer, nonce_step_2, nonce_step_3 })
    }

    /// Compute the next global_work_size many nonces
    pub fn compute_next_nonces(&mut self) -> Result<PlotResult, Error> {
        // step 2
        self.nonce_step_2.set_arg_scl_named("p_startNonce", self.current_nonce)?;
        unsafe { self.nonce_step_2.enq()?; }

        // step 3
        unsafe { self.nonce_step_3.enq()?; }

        // get result
        let mut vec = vec![0u8; self.device_config.global_work_size() as usize * GEN_SIZE];
        unsafe { self.buffer.read(&mut vec).block(true).offset(0).len(self.device_config.global_work_size() as usize * GEN_SIZE).dst_offset(0).enq()?; }

        let start_nonce = self.current_nonce;
        self.current_nonce += self.device_config.global_work_size() as u64;

        return Ok(PlotResult::from_bytes(start_nonce, vec));
    }
}

pub fn continuous_nonce_computer(mut ocl_nonce_computer: OclNonceComputer, receiver: Receiver<u64>, sender: SyncSender<PlotResult>) {
    thread::spawn(move || {
        loop {
            if let Ok(nonce) = receiver.try_recv() {
                ocl_nonce_computer.set_nonce(nonce);
            }

            let next_plot = ocl_nonce_computer.compute_next_nonces().expect("computing nonces failed");
            sender.send(next_plot).expect("sending new nonces failed");
        }
    });

}

