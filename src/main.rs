use eframe::{egui, epaint::{RectShape, Rect, Pos2}};
use tinyfiledialogs::open_file_dialog;
use std::fs::{self, File};

struct Registers {
    v: [u8; 16],
    i: u16,
    dt: u8,
    st: u8,
    pc: u16,
    sp: u8,
    stack: [u16; 16],
    vf: bool,
}

struct Chip8 {
    mem: [u8; 4096],
    reg: Registers,
    dis: [[bool; 64]; 32],
    run: bool,
}

impl Default for Chip8 {
    fn default() -> Self {
        let mut res = Self {
            mem: [0; 4096],
            reg: Registers {
                v: [0; 16],
                i: 0,
                dt: 0,
                st: 0,
                pc: 0x200,
                sp: 0,
                stack: [0; 16],
                vf: false,
            },
            dis: [[false; 64]; 32],
            run: false,
        };

        res.load_int("int.ch8");

        res
    }
}

impl Chip8 {
    fn reset(&mut self) {
        self.mem = [0; 4096];
        self.reg = Registers {
            v: [0; 16],
            i: 0,
            dt: 0,
            st: 0,
            pc: 0x200,
            sp: 0,
            stack: [0; 16],
            vf: false,
        };
        self.dis = [[false; 64]; 32];
        self.run = false;

        self.load_int("int.ch8");
    }

    fn sys_addr(&mut self) {
        
    }

    fn cls(&mut self) {
        //print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
        self.reg.pc += 2;
    }

    fn ret(&mut self) {
        self.reg.pc = self.reg.stack[self.reg.sp as usize];
        self.reg.sp -= 1;
    }

    fn jp_addr(&mut self, nnn: u16) {
        self.reg.pc = nnn;
    }

    fn call_addr(&mut self, nnn: u16) {
        self.reg.sp += 1;
        self.reg.stack[self.reg.sp as usize] = self.reg.pc + 2;
        self.reg.pc = nnn;
    }

    fn se_vx_byte(&mut self, x: u8, kk: u8) {
        if self.reg.v[x as usize] == kk {
            self.reg.pc += 2;
        }
        self.reg.pc += 2;
    }

    fn sne_vx_byte(&mut self, x: u8, kk: u8) {
        if self.reg.v[x as usize] != kk {
            self.reg.pc += 2;
        }
        self.reg.pc += 2;
    }

    fn se_vx_vy(&mut self, x: u8, y: u8) {
        if self.reg.v[x as usize] == self.reg.v[y as usize] {
            self.reg.pc += 2;
        }
        self.reg.pc += 2;
    }

    fn ld_vx_byte(&mut self, x: u8, kk: u8) {
        self.reg.v[x as usize] = kk;
        self.reg.pc += 2;
    }

    fn add_vx_byte(&mut self, x: u8, kk: u8) {
        self.reg.v[x as usize] = self.reg.v[x as usize].overflowing_add(kk).0;
        self.reg.pc += 2;
    }

    fn ld_vx_vy(&mut self, x: u8, y: u8) {
        self.reg.v[x as usize] = self.reg.v[y as usize];
        self.reg.pc += 2;
    }

    fn or_vx_vy(&mut self, x: u8, y: u8) {
        self.reg.v[x as usize] = self.reg.v[x as usize] | self.reg.v[y as usize];
        self.reg.pc += 2;
    }

    fn and_vx_vy(&mut self, x: u8, y: u8) {
        self.reg.v[x as usize] = self.reg.v[x as usize] & self.reg.v[y as usize];
        self.reg.pc += 2;
    }

    fn xor_vx_vy(&mut self, x: u8, y: u8) {
        self.reg.v[x as usize] = self.reg.v[x as usize] ^ self.reg.v[y as usize];
        self.reg.pc += 2;
    }

    fn add_vx_vy(&mut self, x: u8, y: u8) {
        let res = self.reg.v[x as usize].overflowing_add(self.reg.v[y as usize]);
        self.reg.v[x as usize] = res.0;
        self.reg.vf = res.1;
        self.reg.pc += 2;
    }

    fn sub_vx_vy(&mut self, x: u8, y: u8) {
        let res = self.reg.v[x as usize].overflowing_sub(self.reg.v[y as usize]);
        self.reg.v[x as usize] = res.0;
        self.reg.vf = !res.1;
        self.reg.pc += 2;
    }

    fn shr_vx(&mut self, x: u8) {
        if self.reg.v[x as usize] & 1 == 1 {
            self.reg.vf = true;
        } else {
            self.reg.vf = false;
        }

        self.reg.v[x as usize] /= 2;
        self.reg.pc += 2;
    }

    fn subn_vx_vy(&mut self, x: u8, y: u8) {
        if self.reg.v[y as usize] > self.reg.v[x as usize] {
            self.reg.vf = true;
        } else {
            self.reg.vf = false;
        }

        self.reg.v[x as usize] = self.reg.v[y as usize] - self.reg.v[x as usize];
        self.reg.pc += 2;
    }

    fn shl_vx(&mut self, x: u8) {
        let res = self.reg.v[x as usize].overflowing_mul(2);
        self.reg.v[x as usize] = res.0;
        self.reg.vf = res.1;
        self.reg.pc += 2;
    }

    fn sne_vx_vy(&mut self, x: u8, y: u8) {
        if self.reg.v[x as usize] != self.reg.v[y as usize] {
            self.reg.pc += 2;
        }
        self.reg.pc += 2;
    }

    fn ld_i_addr(&mut self, nnn: u16) {
        self.reg.i = nnn;
        self.reg.pc += 2;
    }

    fn jp_v0_addr(&mut self, nnn: u16) {
        self.reg.pc = nnn + self.reg.v[0] as u16;
    }

    fn rnd_vx_byte(&mut self, x: u8, kk: u8) {
        self.reg.v[x as usize] = rand::random::<u8>() & kk;
        self.reg.pc += 2;
    }

    fn drw_vx_vy_n(&mut self, x: u8, y: u8, n: u8) {
        let x_coord = (self.reg.v[x as usize] % 64) as usize;
        let y_coord = (self.reg.v[y as usize] % 32) as usize;

        //println!("{}, {}", x_coord, y_coord);

        let mut erased = false;
        for i in 0..n as usize{
            let byte = self.mem[(self.reg.i + i as u16) as usize];

            for j in 0..8 {
                let bit = byte & (1 << (7 - j)) != 0;

                if self.dis[y_coord + i][x_coord + j] | bit != bit {
                    erased = true;
                }

                self.dis[y_coord + i][x_coord + j] = self.dis[y_coord + i][x_coord + j] ^ bit;
            }
        }
        self.reg.vf = erased;
        //self.blit();
        self.reg.pc += 2;
    }

    fn skp_vx(&mut self, x: u8) {
        let key = self.reg.v[x as usize];

        todo!();
    }

    fn sknp_vx(&mut self, x: u8) {
        let key = self.reg.v[x as usize];

        todo!();
    }

    fn ld_vx_dt(&mut self, x: u8) {
        self.reg.v[x as usize] = self.reg.dt;
        self.reg.pc += 2;
    }

    fn ld_vx_k(&mut self, x: u8) {
        todo!();
    }

    fn ld_dt_vx(&mut self, x: u8) {
        self.reg.dt = self.reg.v[x as usize];
        self.reg.pc += 2;
    }

    fn ld_st_vx(&mut self, x: u8) {
        self.reg.st = self.reg.v[x as usize];
        self.reg.pc += 2;
    }

    fn add_i_vx(&mut self, x: u8) {
        self.reg.i += self.reg.v[x as usize] as u16;
        self.reg.pc += 2;
    }

    fn ld_f_vx(&mut self, x: u8) {
        let res = self.reg.v[x as usize].overflowing_mul(5);
        self.reg.i = res.0 as u16;
        self.reg.pc += 2;
    }

    fn ld_b_vx(&mut self, x: u8) {
        let mut val = self.reg.v[x as usize];
        let ones = val % 10;
        val /= 10;
        let tens = val % 10;
        val /= 10;
        let huns = val % 10;

        self.mem[self.reg.i as usize] = huns;
        self.mem[self.reg.i as usize + 1] = tens;
        self.mem[self.reg.i as usize + 2] = ones;
        self.reg.pc += 2;
    }

    fn ld_i_vx(&mut self, x: u8) {
        for i in 0..x as u16 {
            self.mem[(self.reg.i + i) as usize] = self.reg.v[i as usize];
        }
        self.reg.pc += 2;
    }

    fn ld_vx_i(&mut self, x: u8) {
        for i in 0..x as u16 {
            self.reg.v[i as usize] = self.mem[(self.reg.i + i) as usize];
        }
        self.reg.pc += 2;
    }

    fn cycle(&mut self) {
        let op_addr: usize = self.reg.pc as usize;
        let op_upper: u8 = self.mem[op_addr];
        let op_lower: u8 = self.mem[op_addr + 1];

        let op: u16 = ((op_upper as u16) << 8) | (op_lower as u16);

        let nnn = op & 0x0FFF;
        let n = (op & 0x000F) as u8;
        let x = ((op & 0x0F00) >> 8) as u8;
        let y = ((op & 0x00F0) >> 4) as u8;
        let kk = (op & 0x00FF) as u8;

        let nibbles = ((op & 0xF000) >> 12, x, y, n);
        println!(
            "{:#01x} {:#01x} {:#01x} {:#01x}",
            nibbles.0, nibbles.1, nibbles.2, nibbles.3
        );

        match nibbles {
            (0x0, 0x0, 0xE, 0x0) => self.cls(),
            (0x0, 0x0, 0xE, 0xE) => self.ret(),
            (0x0, _, _, _) => self.sys_addr(),

            (0x1, _, _, _) => self.jp_addr(nnn),
            (0x2, _, _, _) => self.call_addr(nnn),
            (0x3, _, _, _) => self.se_vx_byte(x, kk),
            (0x4, _, _, _) => self.sne_vx_byte(x, kk),
            (0x5, _, _, 0x0) => self.se_vx_vy(x, y),
            (0x6, _, _, _) => self.ld_vx_byte(x, kk),
            (0x7, _, _, _) => self.add_vx_byte(x, kk),

            (0x8, _, _, 0x0) => self.ld_vx_vy(x, y),
            (0x8, _, _, 0x1) => self.or_vx_vy(x, y),
            (0x8, _, _, 0x2) => self.and_vx_vy(x, y),
            (0x8, _, _, 0x3) => self.xor_vx_vy(x, y),
            (0x8, _, _, 0x4) => self.add_vx_vy(x, y),
            (0x8, _, _, 0x5) => self.sub_vx_vy(x, y),
            (0x8, _, _, 0x6) => self.shr_vx(x),
            (0x8, _, _, 0x7) => self.subn_vx_vy(x, y),
            (0x8, _, _, 0xE) => self.shl_vx(x),

            (0x9, _, _, 0x0) => self.sne_vx_vy(x, y),

            (0xA, _, _, _) => self.ld_i_addr(nnn),
            (0xB, _, _, _) => self.jp_v0_addr(nnn),
            (0xC, _, _, _) => self.rnd_vx_byte(x, kk),
            (0xD, _, _, _) => self.drw_vx_vy_n(x, y, n),

            (0xD, _, _, _) => todo!(),

            (0xE, _, 0x9, 0xE) => self.skp_vx(x),
            (0xE, _, 0xA, 0x1) => self.sknp_vx(x),

            (0xF, _, 0x0, 0x7) => self.ld_vx_dt(x),
            (0xF, _, 0x0, 0xA) => self.ld_vx_k(x),

            (0xF, _, 0x1, 0x5) => self.ld_dt_vx(x),
            (0xF, _, 0x1, 0x8) => self.ld_st_vx(x),

            (0xF, _, 0x1, 0xE) => self.add_i_vx(x),
            (0xF, _, 0x2, 0x9) => self.ld_f_vx(x),
            (0xF, _, 0x3, 0x3) => self.ld_b_vx(x),
            (0xF, _, 0x5, 0x5) => self.ld_i_vx(x),
            (0xF, _, 0x6, 0x5) => self.ld_vx_i(x),

            _ => println!("Unknown OP."),
        }
    }

    
    fn blit(&self) {
        print!("\x1B[2J\x1B[1;1H");
        for y in 0..32 {
            for x in 0..64 {
                let bit = self.dis[y][x];
                if bit == true {
                    print!("█");
                } else {
                    print! {" "};
                }
            }
            print!("\n");
        }
    }
    

    fn load_int(&mut self, file_path: &str) {
        let file = fs::read(file_path).unwrap();
        for (idx, &byte) in file.iter().enumerate() {
            self.mem[idx] = byte;
        }
    }

    fn load_rom(&mut self, file_path: &str) {
        let file = fs::read(file_path).unwrap();
        for (idx, &byte) in file.iter().enumerate() {
            self.mem[idx + 0x200] = byte;
        }
    }
}

impl eframe::App for Chip8 {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            eframe::egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let mut cursor_y = 0.0;
                for y in 0..32 {
                    let mut cursor_x = 0.0;
                    for x in 0..64 {
                        let mut rect = Rect::NOTHING;
                        rect.min = Pos2{x: cursor_x, y: cursor_y};
                        rect.max = Pos2{x: cursor_x + 10.0, y: cursor_y + 10.0};

                        let bit = self.dis[y][x];
                        if bit == true {
                            ui.painter().add(egui::Shape::Rect(RectShape::filled(rect, 0.0, egui::Color32::WHITE)));
                        } else {
                            ui.painter().add(egui::Shape::Rect(RectShape::filled(rect, 0.0, egui::Color32::BLACK)));
                        }
                        
                        cursor_x += 10.0;
                    }
                    cursor_y += 10.0;
                }
            });

            ui.add_space(320.0);

            ui.horizontal(|ui| {
                if ui.button("Load ROM").clicked() {
                    let file_path: String;
                    match tinyfiledialogs::open_file_dialog("Open", "", None) {
                        Some(file) => file_path = file,
                        None => return,
                    }

                    self.load_rom(&file_path);
                }

                if ui.button("Reset").clicked() {
                    self.reset();
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Run").clicked() {
                    self.run = true;
                }

                if ui.button("Stop").clicked() {
                    self.run = false;
                }

                if ui.button("1 Clock Cycle").clicked() {
                    self.cycle();
                }
    
                if ui.button("8 Clock Cycles").clicked() {
                    for i in 0..8 {
                        self.cycle();
                    }
                }
            });

            
            ui.label("Registers:");
            ui.label(format!("pc: {}", self.reg.pc));
            ui.label(format!("i: {}", self.reg.i));
            egui::Grid::new("v").striped(true).show(ui, |ui| {
                for i in 0..16 {
                    ui.label(format!("V{:01x}", i));
                }
                ui.end_row();
                for vx in self.reg.v {
                    ui.label(format!("{:01x}", vx));
                }
            });

            ui.label("Stack:");
            ui.label(format!("sp: {}", self.reg.sp));
            egui::Grid::new("stack").striped(true).show(ui, |ui| {
                for v in self.reg.stack {
                    ui.label(format!("{:04x}", v));
                }
            });
        });

        if self.run {
            self.cycle();
            ctx.request_repaint();
        }
    }
}

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Chip-8 Debugger",
        native_options,
        Box::new(|_cc| Box::new(Chip8::default())),
    );
}
