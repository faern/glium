use std::sync::Arc;
use std::mem;

use vertex_buffer::{mod, VertexBuffer, VertexBindings};
use DisplayImpl;

use {libc, gl};

/// 
pub struct VertexArrayObject {
    display: Arc<DisplayImpl>,
    id: gl::types::GLuint,
}

impl VertexArrayObject {
    /// 
    fn new<T>(display: Arc<DisplayImpl>, vertex_buffer: &VertexBuffer<T>,
        program_id: gl::types::GLuint) -> VertexArrayObject
    {
        let (tx, rx) = channel();

        let bindings = vertex_buffer::get_bindings(vertex_buffer).clone();
        let vb_elementssize = vertex_buffer::get_elements_size(vertex_buffer);
        let vertex_buffer = vertex_buffer::get_id(vertex_buffer);

        display.context.exec(proc(gl, state, _, _) {
            unsafe {
                let id: gl::types::GLuint = mem::uninitialized();
                gl.GenVertexArrays(1, mem::transmute(&id));
                tx.send(id);

                gl.BindVertexArray(id);
                state.vertex_array = id;

                // binding vertex buffer
                if state.array_buffer_binding != Some(vertex_buffer) {
                    gl.BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);
                    state.array_buffer_binding = Some(vertex_buffer);
                }

                // binding attributes
                for (name, vertex_buffer::VertexAttrib { offset, data_type, elements_count })
                    in bindings.into_iter()
                {
                    let loc = gl.GetAttribLocation(program_id, name.to_c_str().unwrap());

                    if loc != -1 {
                        match data_type {
                            gl::BYTE | gl::UNSIGNED_BYTE | gl::SHORT | gl::UNSIGNED_SHORT |
                            gl::INT | gl::UNSIGNED_INT =>
                                gl.VertexAttribIPointer(loc as u32,
                                    elements_count as gl::types::GLint, data_type,
                                    vb_elementssize as i32, offset as *const libc::c_void),

                            _ => gl.VertexAttribPointer(loc as u32,
                                    elements_count as gl::types::GLint, data_type, 0,
                                    vb_elementssize as i32, offset as *const libc::c_void)
                        }
                        
                        gl.EnableVertexAttribArray(loc as u32);
                    }
                }
            }
        });

        VertexArrayObject {
            display: display,
            id: rx.recv(),
        }
    }
}

impl Drop for VertexArrayObject {
    fn drop(&mut self) {
        let id = self.id.clone();
        self.display.context.exec(proc(gl, state, _, _) {
            unsafe {
                // unbinding
                if state.vertex_array == id {
                    gl.BindVertexArray(0);
                    state.vertex_array = 0;
                }

                // deleting
                gl.DeleteVertexArrays(1, [ id ].as_ptr());
            }
        });
    }
}

pub fn get_vertex_array_object<T>(display: &Arc<DisplayImpl>, vertex_buffer: &VertexBuffer<T>,
    program_id: gl::types::GLuint) -> gl::types::GLuint
{
    let mut vaos = display.vertex_array_objects.lock();

    let vb_id = vertex_buffer::get_id(vertex_buffer);
    if let Some(value) = vaos.find(&(vb_id, program_id)) {
        return value.id;
    }

    let mut new_vao = VertexArrayObject::new(display.clone(), vertex_buffer, program_id);
    let new_vao_id = new_vao.id;
    vaos.insert((vb_id, program_id), new_vao);
    new_vao_id
}
