// Copyright (C) 2017-2018 Baidu, Inc. All Rights Reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
//
//  * Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
//  * Redistributions in binary form must reproduce the above copyright
//    notice, this list of conditions and the following disclaimer in
//    the documentation and/or other materials provided with the
//    distribution.
//  * Neither the name of Baidu, Inc., nor the names of its
//    contributors may be used to endorse or promote products derived
//    from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
// OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
// LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
// DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
// THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

extern crate dirs;
extern crate sgx_urts;
extern crate secp256k1;
extern crate sgx_types;

use std::fs;
use std::path;
use sgx_types::*;
use sgx_urts::SgxEnclave;
use std::io::{Read, Write};
use secp256k1::key::{PublicKey, SecretKey};

static ENCLAVE_FILE: &'static str = "enclave.signed.so";
static ENCLAVE_TOKEN: &'static str = "enclave.token";

extern {
    fn generate_keypair(
        eid: sgx_enclave_id_t, 
        retval: *mut sgx_status_t, 
        pub_key: *mut PublicKey, 
    ) -> sgx_status_t;

    fn create_sealeddata(
        eid: sgx_enclave_id_t, 
        retval: *mut sgx_status_t, 
        // sealed_log: *mut u8, 
        sealed_log: *mut sgx_sealed_data_t,
        sealed_log_size: *const u32
    ) -> sgx_status_t;
}

fn init_enclave() -> SgxResult<SgxEnclave> {

    let mut launch_token: sgx_launch_token_t = [0; 1024];
    let mut launch_token_updated: i32 = 0;
    // Step 1: try to retrieve the launch token saved by last transaction
    //         if there is no token, then create a new one.
    //
    // try to get the token saved in $HOME */
    let mut home_dir = path::PathBuf::new();
    let use_token = match dirs::home_dir() {
        Some(path) => {
            println!("[+] Home dir is {}", path.display());
            home_dir = path;
            true
        },
        None => {
            println!("[-] Cannot get home dir");
            false
        }
    };

    let token_file: path::PathBuf = home_dir.join(ENCLAVE_TOKEN);;
    if use_token == true {
        match fs::File::open(&token_file) {
            Err(_) => {
                println!("[-] Open token file {} error! Will create one.", token_file.as_path().to_str().unwrap());
            },
            Ok(mut f) => {
                println!("[+] Open token file success! ");
                match f.read(&mut launch_token) {
                    Ok(1024) => {
                        println!("[+] Token file valid!");
                    },
                    _ => println!("[+] Token file invalid, will create new token file"),
                }
            }
        }
    }

    // Step 2: call sgx_create_enclave to initialize an enclave instance
    // Debug Support: set 2nd parameter to 1
    let debug = 1;
    let mut misc_attr = sgx_misc_attribute_t {secs_attr: sgx_attributes_t { flags:0, xfrm:0}, misc_select:0};
    let enclave = try!(SgxEnclave::create(ENCLAVE_FILE,
                                          debug,
                                          &mut launch_token,
                                          &mut launch_token_updated,
                                          &mut misc_attr));

    // Step 3: save the launch token if it is updated
    if use_token == true && launch_token_updated != 0 {
        // reopen the file with write capablity
        match fs::File::create(&token_file) {
            Ok(mut f) => {
                match f.write_all(&launch_token) {
                    Ok(()) => println!("[+] Saved updated launch token!"),
                    Err(_) => println!("[-] Failed to save updated launch token!"),
                }
            },
            Err(_) => {
                println!("[-] Failed to save updated enclave token, but doesn't matter");
            },
        }
    }

    Ok(enclave)
}
/*
 * TODO: Get sealing to work with the PK!
 * TODO: Have the first call the enc. do an ::new, which spits out the PublicKey
 * and a sealed privkey.
 * TODO: Make it a CLI with an -init option and a -sign option. Have the second used 
 * to hash & sign the supplied message usig the sealed priv key.
 * 
 **/
fn main() {
    let enclave = match init_enclave() {
        Ok(r) => {
            println!("[+] Init Enclave Successful {}!", r.geteid());
            r
        },
        Err(x) => {
            println!("[-] Init Enclave Failed {}!", x.as_str());
            return;
        },
    };

    // let mut retval = sgx_status_t::SGX_SUCCESS;
    // let mut pub_key = PublicKey::new();
    // let mut sealed_log = [0u8;32]; // Alloc arr. for secret key
    // let mut log_size: u32 = 1024;
    // let result = unsafe {
    //     generate_keypair(enclave.geteid(), &mut retval, &mut pub_key, &mut sealed_log[0], &mut log_size)
    // };
    // match result {
    //     sgx_status_t::SGX_SUCCESS => {
    //         println!("[+] Key pair successfully generated inside enclave!");
    //         println!("[+] {:?}", pub_key);
    //         // println!("[+] Secret key encrypyed maybe? {:?}", sealed_log)
    //     },
    //     _ => {
    //         println!("[-] ECALL Enclave Failed {}!", result.as_str());
    //         return;
    //     }
    // }

    let mut retval = sgx_status_t::SGX_SUCCESS;
    let mut pub_key = PublicKey::new();
    let result = unsafe {
        generate_keypair(enclave.geteid(), &mut retval, &mut pub_key)
    };
    match result {
        sgx_status_t::SGX_SUCCESS => {
            println!("[+] Key pair successfully generated inside enclave!");
            println!("[+] {:?}", pub_key);
            // println!("[+] Secret key encrypted maybe? {:?}", sealed_log)
        },
        _ => {
            println!("[-] ECALL to enclave failed {}!", result.as_str());
            return;
        }
    };

    let x = std::mem::size_of::<sgx_sealed_data_t>();
    println!("Size of the empty struct {}", x);

    let mut sealed_log_size: u32 = 1024;
    let mut thingy = sgx_sealed_data_t::default();
    
    let result2 = unsafe {
        // create_sealeddata(enclave.geteid(), &mut retval, raw_ptr, &mut sealed_log_size as *const u32) // holy shit this worked!
        // create_sealeddata(enclave.geteid(), &mut retval, sealed_log[0] as *mut u8, sealed_log_size as *const u32) // holy shit this worked too!
        create_sealeddata(enclave.geteid(), &mut retval, &mut thingy, sealed_log_size as *const u32) // holy shit this worked too!
    };

    match result2 {
        sgx_status_t::SGX_SUCCESS => {
            println!("[+] create_sealeddata function call was successful! It returned: {}", result2.as_str());
        },
        _ => {
            println!("[-] ECALL to enclave failed! {}", result2.as_str());
            return;
        }
    };
    
    enclave.destroy();
}