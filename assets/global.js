"use strict";(()=>{var i=(t,e)=>{if(!t)throw new Error(e??"Failed assertion.")};var v=t=>{throw new Error(t??"Reached unreachable code.")};var z={bundle:async t=>{try{return await syscall("artifact_bundle",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},get:async t=>{try{return await syscall("artifact_get",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},B={decode:t=>{try{return syscall("base64_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("base64_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},L={bytes:async t=>{try{return await syscall("blob_bytes",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},new:async t=>{try{return await syscall("blob_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},text:async t=>{try{return await syscall("blob_text",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}};var re={new:t=>{try{return syscall("command_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},se={new:t=>{try{return syscall("directory_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},ne={new:t=>{try{return syscall("file_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},X={new:t=>{try{return syscall("function_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}};var $={decode:t=>{try{return syscall("hex_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("hex_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},N={decode:t=>{try{return syscall("json_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("json_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},ae=t=>{try{return syscall("log",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},H={get:async t=>{try{return await syscall("operation_get",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},run:async t=>{try{return await syscall("operation_run",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},oe={new:t=>{try{return syscall("resource_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},le={new:t=>{try{return syscall("symlink_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},_={decode:t=>{try{return syscall("toml_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("toml_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},M={decode:t=>{try{return syscall("utf8_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("utf8_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},D={decode:t=>{try{return syscall("yaml_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("yaml_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}};var Ae;(r=>(r.decode=n=>B.decode(n),r.encode=n=>B.encode(n)))(Ae||={});var we;(r=>(r.decode=n=>$.decode(n),r.encode=n=>$.encode(n)))(we||={});var Y;(r=>(r.decode=n=>N.decode(n),r.encode=n=>N.encode(n)))(Y||={});var xe;(r=>(r.decode=n=>_.decode(n),r.encode=n=>_.encode(n)))(xe||={});var ke;(r=>(r.decode=n=>M.decode(n),r.encode=n=>M.encode(n)))(ke||={});var Se;(r=>(r.decode=n=>D.decode(n),r.encode=n=>D.encode(n)))(Se||={});var I={get(){return i(this.value),this.value}};var F=(...t)=>p.new(...t),R=(...t)=>g.new(...t),p=class{#e;#t;static new(...e){let r=new p,n=a=>{if(typeof a=="string")for(let o of a.split("/"))o===""||o==="."||(o===".."?r=r.parent():r.#t.push(o));else if(a instanceof p){for(let o=0;o<a.#e;o++)r.parent();r.#t.join(a.#t)}else if(a instanceof g)r.#t.join(a);else if(a instanceof Array)for(let o of a)n(o)};for(let a of e)n(a);return r}constructor(e){this.#e=e?.parents??0,this.#t=e?.subpath??new g}static is(e){return e instanceof p}toSyscall(){return this.toString()}static fromSyscall(e){return p.new(e)}isEmpty(){return this.#e==0&&this.#t.isEmpty()}parents(){return this.#e}subpath(){return this.#t}parent(){return this.#t.isEmpty()?this.#e+=1:this.#t.pop(),this}join(e){e=p.new(e);for(let r=0;r<e.#e;r++)this.parent();return this.#t.join(e.#t),this}extension(){return this.#t.extension()}toSubpath(){if(this.#e>0)throw new Error("Cannot convert to subpath.");return this.#t}toString(){let e="";for(let r=0;r<this.#e;r++)e+="../";return e+=this.#t.toString(),e}};(e=>{let t;(o=>(o.is=s=>s===void 0||typeof s=="string"||s instanceof g||s instanceof e||s instanceof Array&&s.every(e.Arg.is),o.expect=s=>(i((0,o.is)(s)),s),o.assert=s=>{i((0,o.is)(s))}))(t=e.Arg||={})})(p||={});var g=class{#e;static new(...e){return p.new(...e).toSubpath()}constructor(e){this.#e=e??[]}static is(e){return e instanceof g}toSyscall(){return this.toString()}static fromSyscall(e){return R(e)}components(){return[...this.#e]}isEmpty(){return this.#e.length==0}join(e){return this.#e.push(...e.#e),this}push(e){this.#e.push(e)}pop(){this.#e.pop()}extension(){return this.#e.at(-1)?.split(".").at(-1)}toRelpath(){return p.new(this)}toString(){return this.#e.join("/")}};var W=async t=>await h.new(t),h=class{#e;#t;#r;#s;static async new(e){let r=await x(e),n,a,o;if(y.Arg.is(r))n=await K(r),a=!1,o=[];else{if(h.is(r))return r;n=await K(r.blob),a=r.executable??!1,o=r.references??[]}return h.fromSyscall(ne.new({blob:n.toSyscall(),executable:a,references:o.map(s=>m.toSyscall(s))}))}constructor(e){this.#e=e.hash,this.#t=e.blob,this.#r=e.executable,this.#s=e.references}static is(e){return e instanceof h}static expect(e){return i(h.is(e)),e}static assert(e){i(h.is(e))}toSyscall(){return{hash:this.#e,blob:this.#t.toSyscall(),executable:this.#r,references:this.#s}}static fromSyscall(e){return new h({hash:e.hash,blob:y.fromSyscall(e.blob),executable:e.executable,references:e.references})}hash(){return this.#e}blob(){return this.#t}executable(){return this.#r}async references(){return await Promise.all(this.#s.map(m.get))}async bytes(){return await this.blob().bytes()}async text(){return await this.blob().text()}};var G=t=>b.new(t),b=class{#e;static new(e){return new b(e)}constructor(e){this.#e=e}static is(e){return e instanceof b}toSyscall(){return{name:this.#e}}static fromSyscall(e){let r=e.name;return new b(r)}name(){return this.#e}};var ie=async t=>await A.new(t),ce=async t=>await(await A.new(t)).download(),A=class{#e;#t;#r;#s;#n;static async new(e){return A.fromSyscall(oe.new({url:e.url,unpack:e.unpack??!1,checksum:e.checksum??void 0,unsafe:e.unsafe??!1}))}constructor(e){this.#e=e.hash,this.#t=e.url,this.#r=e.unpack??!1,this.#s=e.checksum??void 0,this.#n=e.unsafe??!1}static is(e){return e instanceof A}static expect(e){return i(A.is(e)),e}static assert(e){i(A.is(e))}hash(){return this.#e}toSyscall(){return{hash:this.#e,url:this.#t,unpack:this.#r,checksum:this.#s,unsafe:this.#n}}static fromSyscall(e){return new A({hash:e.hash,url:e.url,unpack:e.unpack,checksum:e.checksum,unsafe:e.unsafe})}async download(){let e=await H.run(C.toSyscall(this));return k.fromSyscall(e)}};var V=async(t,...e)=>{let r=[];for(let n=0;n<t.length-1;n++){let a=t[n];r.push(a);let o=e[n];r.push(o)}return r.push(t[t.length-1]),await j(...r)},j=(...t)=>u.new(...t),u=class{#e;static async new(...e){let r=[],n=o=>{if(u.Component.is(o))r.push(o);else if(o instanceof p||o instanceof g)r.push(o.toString());else if(o instanceof u)r.push(...o.components());else if(o instanceof Array)for(let s of o)n(s)};for(let o of await Promise.all(e.map(x)))n(o);let a=[];for(let o of r){let s=a.at(-1);o!==""&&(typeof s=="string"&&typeof o=="string"?a.splice(-1,1,s+o):a.push(o))}return r=a,r=Te(r),new u(r)}constructor(e){this.#e=e}static is(e){return e instanceof u}static expect(e){return i(u.is(e)),e}static assert(e){i(u.is(e))}static async join(e,...r){let n=await j(e),a=await Promise.all(r.map(s=>j(s)));a=a.filter(s=>s.components().length>0);let o=[];for(let s=0;s<a.length;s++){s>0&&o.push(n);let l=a[s];i(l),o.push(l)}return j(...o)}toSyscall(){return{components:this.#e.map(r=>u.Component.toSyscall(r))}}static fromSyscall(e){let r=e.components.map(n=>u.Component.fromSyscall(n));return new u(r)}components(){return[...this.#e]}};(e=>{let t;(o=>(o.is=s=>typeof s=="string"||m.is(s)||s instanceof b,o.toSyscall=s=>typeof s=="string"?{kind:"string",value:s}:m.is(s)?{kind:"artifact",value:m.toSyscall(s)}:s instanceof b?{kind:"placeholder",value:s.toSyscall()}:v(),o.fromSyscall=s=>{switch(s.kind){case"string":return s.value;case"artifact":return m.fromSyscall(s.value);case"placeholder":return b.fromSyscall(s.value);default:return v()}}))(t=e.Component||={})})(u||={});(e=>{let t;(n=>n.is=a=>e.Component.is(a)||a instanceof p||a instanceof g||a instanceof e||a instanceof Array&&a.every(e.Arg.is))(t=e.Arg||={})})(u||={});var Pe=t=>{let e=t.split(`
`);if(e.length!=1&&(e=e.filter(r=>!/^\s*$/.exec(r)),e=e.map(r=>/^\s*/.exec(r)?.map(a=>a)??[]).flat(),e.length!=0))return e.reduce((r,n)=>{let a=r?.length??0,o=n?.length??0;return a<o?r:n})},Te=t=>{let e;for(let r of t)if(typeof r=="string"){let n=Pe(r);(n&&!e||n&&e&&n.length<e.length)&&(e=n)}if(e){let r=e;t=t.map(n=>typeof n=="string"?n.split(`
`).map(a=>(a.startsWith(r)&&(a=a.replace(r,"")),a)).join(`
`):n)}return t};var Z=async t=>await f.new(t),f=class{#e;#t;static async new(e){let r=await x(e),n,a;if(typeof r=="string")a=r;else if(p.is(r)||g.is(r))a=r.toString();else if(m.is(r))n=r;else if(r instanceof u){i(r.components().length<=2);let[s,l]=r.components();if(typeof s=="string"&&l===void 0)a=s;else if(m.is(s)&&l===void 0)n=s;else if(m.is(s)&&typeof l=="string")n=s,i(l.startsWith("/")),a=l.slice(1);else throw new Error("Invalid template.")}else{if(r instanceof f)return r;if(typeof r=="object"){n=r.artifact;let s=r.path;typeof s=="string"?a=s:g.is(s)&&(a=s.toString())}}let o;return n!==void 0&&a!==void 0?o=await V`${n}/${a}`:n!==void 0&&a===void 0?o=await V`${n}`:n===void 0&&a!==void 0?o=await V`${a}`:o=await V``,f.fromSyscall(le.new({target:o.toSyscall()}))}constructor(e){this.#e=e.hash,this.#t=e.target}static is(e){return e instanceof f}static expect(e){return i(f.is(e)),e}static assert(e){i(f.is(e))}toSyscall(){let e=this.#e,r=this.#t.toSyscall();return{hash:e,target:r}}static fromSyscall(e){let r=e.hash,n=u.fromSyscall(e.target);return new f({hash:r,target:n})}hash(){return this.#e}target(){return this.#t}artifact(){let e=this.#t.components().at(0);if(m.is(e))return e}path(){let[e,r]=this.#t.components();if(typeof e=="string"&&r===void 0)return F(e);if(m.is(e)&&r===void 0)return;if(m.is(e)&&typeof r=="string")return F(r.slice(1));throw new Error("Invalid template.")}async resolve(e){e=e?await Z(e):void 0;let r=e?.artifact();r instanceof f&&(r=await r.resolve());let n=e?.path(),a=this.artifact();a instanceof f&&(a=await a.resolve());let o=this.path();if(a!==void 0&&o===void 0)return a;if(a===void 0&&o!==void 0){if(!(r instanceof c))throw new Error("Expected a directory.");return await r.tryGet((n??F()).join(o).toSubpath())}else if(a!==void 0&&o!==void 0){if(!(a instanceof c))throw new Error("Expected a directory.");return await a.tryGet(o.toSubpath())}else throw new Error("Invalid symlink.")}};var k;(o=>(o.is=s=>s===void 0||typeof s=="boolean"||typeof s=="number"||typeof s=="string"||s instanceof Uint8Array||s instanceof p||s instanceof g||s instanceof y||s instanceof c||s instanceof h||s instanceof f||s instanceof b||s instanceof u||s instanceof P||s instanceof Function||s instanceof A||s instanceof Array||typeof s=="object",o.expect=s=>(i((0,o.is)(s)),s),o.assert=s=>{i((0,o.is)(s))},o.toSyscall=s=>s===void 0?{kind:"null"}:typeof s=="boolean"?{kind:"bool",value:s}:typeof s=="number"?{kind:"number",value:s}:typeof s=="string"?{kind:"string",value:s}:s instanceof Uint8Array?{kind:"bytes",value:s}:s instanceof p?{kind:"relpath",value:s.toSyscall()}:s instanceof g?{kind:"subpath",value:s.toSyscall()}:s instanceof y?{kind:"blob",value:s.toSyscall()}:m.is(s)?{kind:"artifact",value:m.toSyscall(s)}:s instanceof b?{kind:"placeholder",value:s.toSyscall()}:s instanceof u?{kind:"template",value:s.toSyscall()}:C.is(s)?{kind:"operation",value:C.toSyscall(s)}:s instanceof Array?{kind:"array",value:s.map(d=>o.toSyscall(d))}:typeof s=="object"?{kind:"object",value:Object.fromEntries(Object.entries(s).map(([d,w])=>[d,o.toSyscall(w)]))}:v(),o.fromSyscall=s=>{switch(s.kind){case"null":return;case"bool":return s.value;case"number":return s.value;case"string":return s.value;case"bytes":return s.value;case"relpath":return p.fromSyscall(s.value);case"subpath":return g.fromSyscall(s.value);case"blob":return y.fromSyscall(s.value);case"artifact":return m.fromSyscall(s.value);case"placeholder":return b.fromSyscall(s.value);case"template":return u.fromSyscall(s.value);case"operation":return C.fromSyscall(s.value);case"array":return s.value.map(l=>o.fromSyscall(l));case"object":return Object.fromEntries(Object.entries(s.value).map(([l,d])=>[l,o.fromSyscall(d)]));default:return v()}}))(k||={});var q={},ue=t=>{i(t.module.kind==="normal");let e=S.fromSyscall(X.new({packageHash:t.module.value.packageHash,modulePath:t.module.value.modulePath,kind:t.kind??"function",name:t.name,env:{},args:[]}));e.f=t.f;let r=Y.encode({module:t.module,name:t.name});return i(q[r]===void 0),q[r]=e,e};var me=async(t,e,r)=>{I.value=Object.fromEntries(Object.entries(e).map(([s,l])=>[s,k.fromSyscall(l)]));let n=r.map(s=>k.fromSyscall(s)),a=await t(...n);return k.toSyscall(a)},S=class extends globalThis.Function{f;hash;packageHash;modulePath;kind;name;env;args;static new(e){let r=Object.fromEntries(Object.entries(e.env??{}).map(([o,s])=>[o,k.toSyscall(s)])),n=(e.args??[]).map(o=>k.toSyscall(o)),a=S.fromSyscall(X.new({packageHash:e.function.packageHash,modulePath:e.function.modulePath.toSyscall(),kind:e.function.kind,name:e.function.name,env:r,args:n}));return a.f=e.function.f,a}constructor(e){return super(),this.f=e.f,this.hash=e.hash,this.packageHash=e.packageHash,this.modulePath=R(e.modulePath),this.kind=e.kind,this.name=e.name,this.env=e.env,this.args=e.args,new Proxy(this,{apply:async(r,n,a)=>{let o=S.new({function:r,args:await Promise.all(a.map(x)),env:I.value}),s=await H.run(C.toSyscall(o));return k.fromSyscall(s)}})}static is(e){return e instanceof S}static expect(e){return i(S.is(e)),e}static assert(e){i(S.is(e))}toSyscall(){let e=this.hash,r=this.packageHash,n=this.modulePath.toString(),a=this.kind,o=this.name,s=this.env?Object.fromEntries(Object.entries(this.env).map(([d,w])=>[d,k.toSyscall(w)])):void 0,l=this.args?this.args.map(d=>k.toSyscall(d)):void 0;return{hash:e,packageHash:r,modulePath:n,kind:a,name:o,env:s,args:l}}static fromSyscall(e){let r=e.hash,n=e.packageHash,a=e.modulePath,o=e.kind,s=e.name,l=e.env!==void 0?Object.fromEntries(Object.entries(e.env).map(([w,T])=>[w,k.fromSyscall(T)])):void 0,d=e.args!==void 0?e.args.map(w=>k.fromSyscall(w)):void 0;return new S({hash:r,packageHash:n,modulePath:a,kind:o,name:s,env:l,args:d})}};var C;(n=>(n.is=a=>a instanceof P||a instanceof S||a instanceof A,n.toSyscall=a=>a instanceof P?{kind:"command",value:a.toSyscall()}:a instanceof S?{kind:"function",value:a.toSyscall()}:a instanceof A?{kind:"resource",value:a.toSyscall()}:v(),n.fromSyscall=a=>{switch(a.kind){case"command":return P.fromSyscall(a.value);case"function":return S.fromSyscall(a.value);case"resource":return A.fromSyscall(a.value);default:return v()}}))(C||={});var pe=async t=>await P.new(t),fe=async t=>await(await P.new(t)).run(),ye=G("output"),P=class{#e;#t;#r;#s;#n;#a;#o;#l;#i;static async new(e){let r=await x(e),n=r.system,a=await j(r.executable),o=Object.fromEntries(await Promise.all(Object.entries(r.env??{}).map(async([E,Q])=>[E,await j(Q)]))),s=Object.fromEntries(Object.entries(o).map(([E,Q])=>[E,Q.toSyscall()])),l=await Promise.all((r.args??[]).map(async E=>(await j(E)).toSyscall())),d=r.checksum??void 0,w=r.unsafe??!1,T=r.network??!1,U=r.hostPaths??[];return P.fromSyscall(re.new({system:n,executable:a.toSyscall(),env:s,args:l,checksum:d,unsafe:w,network:T,hostPaths:U}))}constructor(e){this.#e=e.hash,this.#t=e.system,this.#r=e.executable,this.#s=e.env,this.#n=e.args,this.#a=e.checksum,this.#o=e.unsafe,this.#l=e.network,this.#i=e.hostPaths}toSyscall(){let e=this.#e,r=this.#t,n=this.#r.toSyscall(),a=Object.fromEntries(Object.entries(this.#s).map(([T,U])=>[T,U.toSyscall()])),o=this.#n.map(T=>T.toSyscall()),s=this.#a,l=this.#o,d=this.#l,w=this.#i;return{hash:e,system:r,executable:n,env:a,args:o,checksum:s,unsafe:l,network:d,hostPaths:w}}static fromSyscall(e){let r=e.hash,n=e.system,a=u.fromSyscall(e.executable),o=Object.fromEntries(Object.entries(e.env).map(([U,E])=>[U,u.fromSyscall(E)])),s=e.args.map(U=>u.fromSyscall(U)),l=e.checksum,d=e.unsafe,w=e.network,T=e.hostPaths;return new P({hash:r,system:n,executable:a,env:o,args:s,checksum:l,unsafe:d,network:w,hostPaths:T})}hash(){return this.#e}async run(){let e=await H.run(C.toSyscall(this));return k.fromSyscall(e)}};var x=async t=>{if(t=await t,t===void 0||typeof t=="boolean"||typeof t=="number"||typeof t=="string"||t instanceof Uint8Array||t instanceof p||t instanceof g||t instanceof y||t instanceof c||t instanceof h||t instanceof f||t instanceof b||t instanceof u||t instanceof P||t instanceof Function||t instanceof A)return t;if(t instanceof Array)return await Promise.all(t.map(e=>x(e)));if(typeof t=="object")return Object.fromEntries(await Promise.all(Object.entries(t).map(async([e,r])=>[e,await x(r)])));throw new Error("Invalid value to resolve.")};var K=async t=>await y.new(t),y=class{#e;static async new(e){let r=await x(e),n;if(r instanceof Uint8Array||typeof r=="string")n=r;else return r;return y.fromSyscall(await L.new(n))}constructor(e){this.#e=e.hash}static is(e){return e instanceof y}static expect(e){return i(y.is(e)),e}static assert(e){i(y.is(e))}toSyscall(){return{hash:this.#e}}static fromSyscall(e){let r=e.hash;return new y({hash:r})}hash(){return this.#e}async bytes(){return await L.bytes(this.toSyscall())}async text(){return await L.text(this.toSyscall())}};(e=>{let t;(o=>(o.is=s=>s instanceof Uint8Array||typeof s=="string"||s instanceof e,o.expect=s=>(i((0,o.is)(s)),s),o.assert=s=>{i((0,o.is)(s))}))(t=e.Arg||={})})(y||={});var he=async(...t)=>await c.new(...t),c=class{#e;#t;static async new(...e){let r={};for(let n of await Promise.all(e.map(x)))if(n!==void 0){if(n instanceof c)for(let[a,o]of Object.entries(await n.entries())){let s=r[a];s instanceof c&&o instanceof c&&(o=await c.new(s,o)),r[a]=o}else if(typeof n=="object")for(let[a,o]of Object.entries(n)){let[s,...l]=R(a).components();if(s===void 0)throw new Error("The path must have at least one component.");let d=s,w=r[d];if(w instanceof c||(w=void 0),l.length>0){let T=R(l).toString(),U=await c.new(w,{[T]:o});r[d]=U}else if(o===void 0)delete r[d];else if(y.Arg.is(o)){let T=await W(o);r[d]=T}else if(h.is(o)||f.is(o))r[d]=o;else{let T=await c.new(w,o);r[d]=T}}}return c.fromSyscall(se.new({entries:Object.fromEntries(Object.entries(r).map(([n,a])=>[n,m.toSyscall(a)]))}))}constructor(e){this.#e=e.hash,this.#t=e.entries}static is(e){return e instanceof c}static expect(e){return i(c.is(e)),e}static assert(e){i(c.is(e))}toSyscall(){return{hash:this.#e,entries:this.#t}}static fromSyscall(e){let r=e.hash,n=e.entries;return new c({hash:r,entries:n})}hash(){return this.#e}async get(e){let r=await this.tryGet(e);return i(r,`Failed to get the directory entry "${e}".`),r}async tryGet(e){let r=R(),n=this,a=this;for(let o of R(e).components()){if(r.push(o),n instanceof f){n.artifact()&&(a=n,r=R());let l=await n.resolve(V`${a}/${r}/..`);if(l===void 0)return;n=l}if(n instanceof h)return;let s=n.#t[o];if(!s)return;n=await m.get(s)}if(n instanceof f){let o=await n.resolve(V`${this}/${e}/..`);if(o===void 0)return;n=o}return n}async entries(){let e={};for await(let[r,n]of this)e[r]=n;return e}async bundle(){let e=m.fromSyscall(await z.bundle(m.toSyscall(this)));return i(c.is(e)),e}async*walk(){for await(let[e,r]of this)if(yield[R(e),r],c.is(r))for await(let[n,a]of r.walk())yield[R(e).join(n),a]}*[Symbol.iterator](){for(let[e,r]of Object.entries(this.#t))yield[e,r]}async*[Symbol.asyncIterator](){for(let e of Object.keys(this.#t))yield[e,await this.get(e)]}};var m;(s=>(s.is=l=>l instanceof c||l instanceof h||l instanceof f,s.expect=l=>(i((0,s.is)(l)),l),s.assert=l=>{i((0,s.is)(l))},s.get=async l=>s.fromSyscall(await z.get(l)),s.toSyscall=l=>l instanceof c?{kind:"directory",value:l.toSyscall()}:l instanceof h?{kind:"file",value:l.toSyscall()}:l instanceof f?{kind:"symlink",value:l.toSyscall()}:v(),s.fromSyscall=l=>{switch(l.kind){case"directory":return c.fromSyscall(l.value);case"file":return h.fromSyscall(l.value);case"symlink":return f.fromSyscall(l.value);default:return v()}}))(m||={});var de=(t,e)=>({callSites:e.map(n=>({typeName:n.getTypeName(),functionName:n.getFunctionName(),methodName:n.getMethodName(),fileName:n.getFileName(),lineNumber:n.getLineNumber(),columnNumber:n.getColumnNumber(),isEval:n.isEval(),isNative:n.isNative(),isConstructor:n.isConstructor(),isAsync:n.isAsync(),isPromiseAll:n.isPromiseAll(),promiseIndex:n.getPromiseIndex()}))});var ge=async t=>{i(t.module.kind==="normal");let e=await m.get(t.module.value.packageHash);c.assert(e);let r=R(t.module.value.modulePath).toRelpath().parent().join(t.path).toSubpath();return e.get(r)};var ee=(...t)=>{let e=t.map(r=>Re(r)).join(" ");ae(e)},Re=t=>J(t,new WeakSet),J=(t,e)=>{switch(typeof t){case"string":return`"${t}"`;case"number":return t.toString();case"boolean":return t?"true":"false";case"undefined":return"undefined";case"object":return t===null?"null":ve(t,e);case"function":return`(function "${t.name??"(anonymous)"}")`;case"symbol":return"(symbol)";case"bigint":return t.toString()}},ve=(t,e)=>{if(e.has(t))return"(circular)";if(e.add(t),t instanceof Array)return`[${t.map(r=>J(r,e)).join(", ")}]`;if(t instanceof Error)return t.stack??"";if(t instanceof Promise)return"(promise)";if(t instanceof p)return`(tg.relpath ${t.toString()})`;if(t instanceof g)return`(tg.subpath ${t.toString()})`;if(t instanceof y)return`(tg.blob ${t.hash()})`;if(t instanceof c)return`(tg.directory ${t.hash()})`;if(t instanceof h)return`(tg.file ${t.hash()})`;if(t instanceof f)return`(tg.symlink ${t.hash()})`;if(t instanceof b)return`(tg.placeholder "${t.name()}")`;if(t instanceof u)return`(tg.template "${t.components().map(n=>typeof n=="string"?n:`\${${J(n,e)}}`).join("")}")`;if(t instanceof P)return`(tg.command "${t.hash()}")`;if(t instanceof S)return`(tg.function "${t.hash}")`;if(t instanceof A)return`(tg.resource "${t.hash()}")`;{let r="";t.constructor!==void 0&&t.constructor.name!=="Object"&&(r=`${t.constructor.name} `);let n=Object.entries(t).map(([a,o])=>`${a}: ${J(o,e)}`);return`${r}{ ${n.join(", ")} }`}};var be=t=>{if(typeof t=="string")return t;{let{arch:e,os:r}=t;return`${e}_${r}`}},te;(n=>(n.is=a=>a==="amd64_linux"||a==="arm64_linux"||a==="amd64_macos"||a==="arm64_macos",n.arch=a=>{switch(a){case"amd64_linux":case"amd64_macos":return"amd64";case"arm64_linux":case"arm64_macos":return"arm64";default:throw new Error("Invalid system.")}},n.os=a=>{switch(a){case"amd64_linux":case"arm64_linux":return"linux";case"amd64_macos":case"arm64_macos":return"macos";default:throw new Error("Invalid system.")}}))(te||={});Object.defineProperties(Error,{prepareStackTrace:{value:de}});var Ce={log:ee};Object.defineProperties(globalThis,{console:{value:Ce}});var Oe={Artifact:m,Blob:y,Directory:c,File:h,Function:S,Placeholder:b,Relpath:p,Subpath:g,Symlink:f,System:te,Template:u,Value:k,base64:B,blob:K,command:pe,directory:he,download:ce,entrypoint:me,env:I,file:W,function:ue,hex:$,include:ge,json:N,log:ee,output:ye,placeholder:G,functions:q,relpath:F,resolve:x,resource:ie,run:fe,subpath:R,symlink:Z,system:be,template:j,toml:_,utf8:M,yaml:D};Object.defineProperties(globalThis,{tg:{value:Oe},t:{value:V}});})();
//# sourceMappingURL=global.js.map
