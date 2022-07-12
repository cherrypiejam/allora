use core::str::from_utf8;

use crate::mutex::Mutex;
use crate::uart::UART;
use crate::virtio::{VirtIOBlk, VirtIOEntropy};

pub struct Shell<'a, 'b> {
    pub blk: &'a Mutex<Option<VirtIOBlk<'b>>>,
    pub entropy: &'a Mutex<Option<VirtIOEntropy<'b>>>,
}

impl<'a, 'b> Shell<'a, 'b> {
    fn get_random<F: FnMut(&[u8])>(&mut self, mut f: F) {
        let mut data: [u8; 16] = [0; 16];
        self.entropy.map(|e| e.read(&mut data));
        // self.entropy.map(|e| e.readf(&mut data, &mut f));
        f(b"Random: ");
        f(&data);
    }

    fn write_random<F: FnMut(&[u8])>(&mut self, words: &mut dyn Iterator<Item = &[u8]>, mut f: F) {
        let mut sector = words
            .next()
            .and_then(|sec| from_utf8(sec).ok())
            .and_then(|sec| sec.parse::<u64>().ok())
            .unwrap_or(0);
        let mut len = words
            .next()
            .and_then(|len| from_utf8(len).ok())
            .and_then(|len| len.parse::<usize>().ok())
            .unwrap_or(0);
        while len > 0 {
            let mut outdata: [u8; 512] = [0; 512];
            let curlen = core::cmp::min(512, len);
            {
                let curbuf = &mut outdata[..curlen];
                self.entropy.map(|e| e.read(curbuf));
                for b in curbuf.iter_mut() {
                    *b = ((*b as u32 * 100) / 272 + 32) as u8;
                }
            }
            self.blk.map(|blk| blk.write(sector, &outdata));
            sector += 1;
            len -= curlen;
        }
        f(b"done");
    }

    fn read<F: FnMut(&[u8])>(&mut self, words: &mut dyn Iterator<Item = &[u8]>, mut f: F) {
        let sector = words
            .next()
            .and_then(|sec| from_utf8(sec).ok())
            .and_then(|sec| sec.parse::<u64>().ok())
            .unwrap_or(0);
        let mut len = words
            .next()
            .and_then(|len| from_utf8(len).ok())
            .and_then(|len| len.parse::<usize>().ok())
            .unwrap_or(512);
        let mut data: [u8; 512] = [0; 512];
        loop {
            self.blk.map(|blk| blk.read(sector, &mut data));
            if len > 512 {
                f(&data);
                len -= 512;
            } else {
                f(&data[..len]);
                break;
            }
        }
    }

    /*fn write<F: FnMut(&[u8])>(&mut self, words: &mut dyn Iterator<Item = &[u8]>, mut f: F) {
        let mut sector = words
            .next()
            .and_then(|sec| from_utf8(sec).ok())
            .and_then(|sec| sec.parse::<u64>().ok())
            .unwrap_or(0);
        let mut len = words
            .next()
            .and_then(|len| from_utf8(len).ok())
            .and_then(|len| len.parse::<usize>().ok())
            .unwrap_or(0);
        while len > 0 {
            let mut outdata: [u8; 512] = [0; 512];
            let curlen = core::cmp::min(512, len);
            {
                let curbuf = &mut outdata[..curlen];
                for b in curbuf.iter_mut() {
                    *b = self.uart.read_byte();
                    if *b == b'\r' {
                        *b = b'\n';
                    }
                    self.uart.write_byte(*b);
                }
            }
            self.blk.write(sector, &outdata);
            sector += 1;
            len -= curlen;
        }
    }*/

    pub fn do_line<F>(&mut self, line: &[u8], mut f: F) -> bool
    where
        F: FnMut(&[u8]),
    {
        let line = line
            .split(|c| *c == b'\n' || *c == b'\r')
            .next()
            .unwrap_or(&[]);
        let mut words = line.split(|c| *c == b' ');
        match words.next() {
            Some(b"rand") => {
                self.get_random(f);
            }
            Some(b"writerand") => {
                self.write_random(&mut words, f);
            }
            Some(b"read") => {
                self.read(&mut words, f);
            }
            /*Some(b"write") => {
                self.write(&mut words, f);
            }*/
            Some(b"exit") => {
                return true;
            }
            _ => {
                f(b"Unknown command \"");
                f(line);
                f(b"\"");
            }
        }
        return false;
    }
}

pub fn main(uart: &Mutex<Option<UART>>, app: &mut Shell) {
    loop {
        uart.map(|u| u.write_bytes(b"$> "));
        let mut buf = [0; 1024];
        let line = uart.map(|u| u.read_line(&mut buf, true)).unwrap_or(b"");
        if app.do_line(line, |output| {
            uart.map(|u| u.write_bytes(output));
        }) {
            break;
        }
        uart.map(|u| u.write_byte(b'\n'));
    }
}
