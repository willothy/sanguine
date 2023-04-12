#[derive(Debug, Clone)]
pub enum Alignment {
    Start,
    Middle,
    End,
}

impl Default for Alignment {
    fn default() -> Self {
        Alignment::Start
    }
}

pub trait Align
where
    Self: Sized,
{
    fn align(self, h_align: Alignment, v_align: Alignment) -> Box<Self>;
    fn align_h(self, h_align: Alignment) -> Box<Self>;
    fn align_v(self, v_align: Alignment) -> Box<Self>;
    fn topleft(self) -> Box<Self> {
        self.align_h(Alignment::Start).align_v(Alignment::Start)
    }
    fn topcenter(self) -> Box<Self> {
        self.align_h(Alignment::Middle).align_v(Alignment::Start)
    }
    fn topright(self) -> Box<Self> {
        self.align_h(Alignment::End).align_v(Alignment::Start)
    }
    fn centerleft(self) -> Box<Self> {
        self.align_h(Alignment::Start).align_v(Alignment::Middle)
    }
    fn center(self) -> Box<Self> {
        self.align_h(Alignment::Middle).align_v(Alignment::Middle)
    }
    fn centerright(self) -> Box<Self> {
        self.align_h(Alignment::End).align_v(Alignment::Middle)
    }
    fn bottomleft(self) -> Box<Self> {
        self.align_h(Alignment::Start).align_v(Alignment::End)
    }
    fn bottomcenter(self) -> Box<Self> {
        self.align_h(Alignment::Middle).align_v(Alignment::End)
    }
    fn bottomright(self) -> Box<Self> {
        self.align_h(Alignment::End).align_v(Alignment::End)
    }
}
