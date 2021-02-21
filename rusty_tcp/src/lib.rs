pub enum RunningMode {
    DebugStem7482,
    Tp3,
}

pub struct Config {
    pub data: [u8; 16],
}

impl Config {
    pub fn bin(&self) -> bool {
        match self.data[0] {
            0 => {
                println!("Bin is False.");
                false
            },
            1 => {
                println!("Bin is True.");
                true
            },
            _ => panic!("Binning choice must be 0 | 1."),
        }
    }

    pub fn bytedepth(&self) -> usize {
        match self.data[1] {
            0 => {
                println!("Bitdepth is 8.");
                1
            },
            1 => {
                println!("Bitdepth is 16.");
                2
            },
            2 => {
                println!("Bitdepth is 32.");
                4
            },
            _ => panic!("Bytedepth must be  1 | 2 | 4."),
        }
    }

    pub fn cumul(&self) -> bool {
        match self.data[2] {
            0 => {
                println!("Cumulation mode is OFF.");
                false
            },
            1 => {
                println!("Cumulation mode is ON.");
                true
            },
            _ => panic!("Cumulation must be 0 | 1."),
        }
    }

    pub fn is_spim(&self) -> bool {
        match self.data[3] {
            0 => {
                println!("Spim is OFF.");
                false
            },
            1 => {
                println!("Spim is ON.");
                true
                    },
            _ => panic!("Spim config must be 0 | 1."),
        }
    }

    pub fn xspim_size(&self) -> usize {
        let x = (self.data[4] as usize)<<8 | (self.data[5] as usize);
        println!("X Spim size is {}", x);
        x
    }
    
    pub fn yspim_size(&self) -> usize {
        let y = (self.data[6] as usize)<<8 | (self.data[7] as usize);
        println!("Y Spim size is {}", y);
        y
    }

}

pub enum TdcType {
    TdcOneRisingEdge,
    TdcOneFallingEdge,
    TdcTwoRisingEdge,
    TdcTwoFallingEdge,
}

impl TdcType {
    pub fn associate_value(&self) -> u8 {
        match *self {
            TdcType::TdcOneRisingEdge => 15,
            TdcType::TdcOneFallingEdge => 10,
            TdcType::TdcTwoRisingEdge => 14,
            TdcType::TdcTwoFallingEdge => 11,
        }
    }

    pub fn associate_string(&self) -> &str {
        match *self {
            TdcType::TdcOneRisingEdge => "One_Rising",
            TdcType::TdcOneFallingEdge => "One_Falling",
            TdcType::TdcTwoRisingEdge => "Two_Rising",
            TdcType::TdcTwoFallingEdge => "Two_Falling",
        }
    }

}

pub struct Packet<'a> {
    pub chip_index: u8,
    pub data: &'a [u8],
}

impl Packet<'_> {
    
    pub fn x(&self) -> usize {
        let temp = ((((self.data[6] & 224))>>4 | ((self.data[7] & 15))<<4) | (((self.data[5] & 112)>>4)>>2)) as usize;
        match self.chip_index {
            0 => 255 - temp,
            1 => 255 * 4 - temp,
            2 => 255 * 3 - temp,
            3 => 255 * 2 - temp,
            _ => temp,
        }
    }
    
    pub fn x_unmod(&self) -> usize {
        !((((self.data[6] & 224))>>4 | ((self.data[7] & 15))<<4) | (((self.data[5] & 112)>>4)>>2)) as usize
    }
    
    pub fn y(&self) -> usize {
        (   ( ((self.data[5] & 128))>>5 | ((self.data[6] & 31))<<3 ) | ( (((self.data[5] & 112)>>4)) & 3 )   ) as usize
    }

    pub fn id(&self) -> u8 {
        (self.data[7] & 240) >> 4
    }

    pub fn spidr(&self) -> u16 {
        (self.data[0] as u16) | (self.data[1] as u16)<<8
    }

    pub fn ftoa(&self) -> u8 {
        self.data[2] & 15
    }

    pub fn tot(&self) -> u16 {
        ((self.data[2] & 240) as u16)>>4 | ((self.data[3] & 63) as u16)<<4
    }

    pub fn toa(&self) -> u16 {
        ((self.data[3] & 192) as u16)>>6 | (self.data[4] as u16)<<2 | ((self.data[5] & 15) as u16)<<10
    }

    pub fn ctoa(&self) -> u32 {
        let toa = ((self.data[3] & 192) as u32)>>6 | (self.data[4] as u32)<<2 | ((self.data[5] & 15) as u32)<<10;
        let ftoa = (self.data[2] & 15) as u32;
        (toa << 4) | (!ftoa & 15)
    }
    
    pub fn electron_time(&self) -> f64 {
        let spidr = (self.data[0] as u16) | (self.data[1] as u16)<<8;
        let toa = ((self.data[3] & 192) as u32)>>6 | (self.data[4] as u32)<<2 | ((self.data[5] & 15) as u32)<<10;
        let ftoa = (self.data[2] & 15) as u32;
        let ctoa = (toa << 4) | (!ftoa & 15);
        ((spidr as f64) * 25.0 * 16384.0 + (ctoa as f64) * 25.0 / 16.0) / 1e9
    }

    pub fn tdc_coarse(&self) -> u64 {
        ((self.data[1] & 254) as u64)>>1 | ((self.data[2]) as u64)<<7 | ((self.data[3]) as u64)<<15 | ((self.data[4]) as u64)<<23 | ((self.data[5] & 15) as u64)<<31
    }
    
    pub fn tdc_fine(&self) -> u8 {
        (self.data[0] & 224)>>5 | (self.data[1] & 1)<<3
    }

    pub fn tdc_counter(&self) -> u16 {
        ((self.data[5] & 240) as u16) >> 4 | (self.data[6] as u16) << 4
    }

    pub fn tdc_type(&self) -> u8 {
        self.data[7] & 15 
    }
    
    pub fn tdc_type_as_enum(&self) -> Result<TdcType, &str> {
        match self.data[7] & 15 {
            15 => Ok(TdcType::TdcOneRisingEdge),
            10 => Ok(TdcType::TdcOneFallingEdge),
            14 => Ok(TdcType::TdcTwoRisingEdge),
            11 => Ok(TdcType::TdcTwoFallingEdge),
            _ => Err("Bad TDC receival"),
        }
    }

    pub fn is_tdc_type_oneris(&self) -> Result<bool, &str> {
        match self.data[7] & 15 {
            15 => Ok(true),
            10 | 14 | 11 => Ok(false),
            _ => Err("Bad TDC receival"),
        }
    }
    
    pub fn is_tdc_type_onefal(&self) -> Result<bool, &str> {
        match self.data[7] & 15 {
            10 => Ok(true),
            15 | 14 | 11 => Ok(false),
            _ => Err("Bad TDC receival"),
        }
    }
    
    pub fn is_tdc_type_tworis(&self) -> Result<bool, &str> {
        match self.data[7] & 15 {
            14 => Ok(true),
            10 | 15 | 11 => Ok(false),
            _ => Err("Bad TDC receival"),
        }
    }

    pub fn is_tdc_type_twofal(&self) -> Result<bool, &str> {
        match self.data[7] & 15 {
            11 => Ok(true),
            10 | 14 | 15 => Ok(false),
            _ => Err("Bad TDC receival"),
        }
    }
    
    pub fn calc_elec_time(spidr: u16, toa: u16, ftoa: u8) -> f64 {
        let ctoa = ((toa as u32 )<<4) | (!(ftoa as u32) & 15);
        ((spidr as f64) * 25.0 * 16384.0 + (ctoa as f64) * 25.0 / 16.0) / 1e9
    }

    pub fn tdc_time(coarse: u64, fine: u8) -> f64 {
        (coarse as f64) * (1.0/320e6) + (fine as f64) * 260e-12
    }

    pub fn append_to_array(data: &mut [u8], index:usize, bytedepth: usize) -> bool{
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
                false
            },
            2 => {
                data[index+1] = data[index+1].wrapping_add(1);
                if data[index+1]==0 {
                    data[index] = data[index].wrapping_add(1);
                    true
                } else {
                    false
                }
            },
            1 => {
                data[index] = data[index].wrapping_add(1);
                if data[index]==0 {
                    true
                } else {
                    false
                }
            },
            _ => {panic!("Bytedepth must be 1 | 2 | 4.");},
        }
    }

    pub fn append_to_index_array(data: &mut Vec<u8>, index: usize) {
        data.push(((index & 4_278_190_080)>>24) as u8);
        data.push(((index & 16_711_680)>>16) as u8);
        data.push(((index & 65_280)>>8) as u8);
        data.push((index & 255) as u8);
    }
}

/*
pub mod StartOptions {
    pub enum rRunningMode {
        DebugStem7482,
        Tp3,
    }
}
*/
