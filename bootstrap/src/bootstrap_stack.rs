//! Bootstrap stack
//!
//! A bootstrap stack is structured as follow :
//!
//!     j--------------------j  < 0xaaaa0000 = BootstrapStack.stack_address
//!     |                    |
//!     |                    |
//!     |     PAGE GUARD     |
//!     |                    |
//!     |                    |
//!     j--------------------j
//!     |                    |
//!     |                    |
//!     |        AAA         |
//!     |        |||         |
//!     |                    |
//!     j--------------------j
//!     |                    |
//!     |       STACK        |
//!     |                    |
//!     | j----------------j |
//!     | |  poison value  | |
//!     j-j----------------j-j < 0xaaaaffff
//!          No Page Guard
//!
//!  Since the stack is several pages long, we must ensure the stack respects some alignment
//!  in order to be able to find its bottom from any page.
//!
//! Must be consistent with KernelStack, as kernel considers it's already running on a KernelStack.

use ::core::mem::size_of;
use paging::*;
use address::VirtualAddress;

/// The size of a kernel stack, not accounting for the page guard
pub const STACK_SIZE: usize            = 4;
pub const STACK_SIZE_WITH_GUARD: usize = STACK_SIZE + 1;

/// The alignment of the stack. ceil(log2(STACK_SIZE_WITH_GUARD * PAGE_SIZE))
const STACK_ALIGNEMENT: usize = 15;

/// A structure representing a kernel stack
#[derive(Debug)]
pub struct BootstrapStack {
    stack_address: VirtualAddress // This falls in the page guard
}

impl BootstrapStack {
    /// Allocates the bootstrap stack
    pub fn allocate_stack() -> Option<BootstrapStack> {
        let mut tables = ACTIVE_PAGE_TABLES.lock();
        tables.find_available_virtual_space_aligned::<KernelLand>(STACK_SIZE_WITH_GUARD, STACK_ALIGNEMENT)
            .map(|va| {
                tables.map_range_allocate(VirtualAddress(va.addr() + PAGE_SIZE), STACK_SIZE,
                                          EntryFlags::WRITABLE);
                tables.map_page_guard(va);

                let mut me = BootstrapStack { stack_address: va };
                unsafe {
                    // This is safe because va points to valid memory
                    me.create_poison_pointers();
                };
                me
            })
    }

    /// We keep 2 poison pointers for fake saved ebp and saved esp at the base of the stack
    const STACK_POISON_SIZE: usize = 2 * size_of::<usize>();

    /// Puts two poisons pointers at the base of the stack for the saved ebp and saved eip
    unsafe fn create_poison_pointers(&mut self) {
        let saved_eip: *mut usize = (self.stack_address.addr() + STACK_SIZE_WITH_GUARD * PAGE_SIZE
                                                               - size_of::<usize>()
                                    ) as *mut usize;
        let saved_ebp: *mut usize = saved_eip.offset(-1);
        *saved_eip = 0x00000000;
        *saved_ebp = 0x00000000;
    }

    /// Get the address of the beginning of usable stack.
    /// Used for initializing $esp and $ebp of a newborn process
    /// Points to the last poison pointer, for saved $ebp
    pub fn get_stack_start(&self) -> usize {
         self.stack_address.addr() + STACK_SIZE_WITH_GUARD * PAGE_SIZE
                                   - Self::STACK_POISON_SIZE
    }
}