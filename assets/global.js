"use strict";(()=>{var i=(t,e)=>{if(!t)throw new Error(e??"Failed assertion.")};var O=t=>{throw new Error(t??"Reached unreachable code.")};var B={get(){return i(this.value),this.value}};var G=(...t)=>f.new(...t),S=(...t)=>g.new(...t),f=class{#e;#t;static new(...e){let r=new f,a=n=>{if(typeof n=="string")for(let o of n.split("/"))o===""||o==="."||(o===".."?r=r.parent():r.#t.push(o));else if(n instanceof f){for(let o=0;o<n.#e;o++)r.parent();r.#t.join(n.#t)}else if(n instanceof g)r.#t.join(n);else if(n instanceof Array)for(let o of n)a(o)};for(let n of e)a(n);return r}constructor(e){this.#e=e?.parents??0,this.#t=e?.subpath??new g}static is(e){return e instanceof f}toSyscall(){return this.toString()}static fromSyscall(e){return f.new(e)}isEmpty(){return this.#e==0&&this.#t.isEmpty()}parents(){return this.#e}subpath(){return this.#t}parent(){return this.#t.isEmpty()?this.#e+=1:this.#t.pop(),this}join(e){e=f.new(e);for(let r=0;r<e.#e;r++)this.parent();return this.#t.join(e.#t),this}extension(){return this.#t.extension()}toSubpath(){if(this.#e>0)throw new Error("Cannot convert to subpath.");return this.#t}toString(){let e="";for(let r=0;r<this.#e;r++)e+="../";return e+=this.#t.toString(),e}};(e=>{let t;(o=>(o.is=s=>s===void 0||typeof s=="string"||s instanceof g||s instanceof e||s instanceof Array&&s.every(e.Arg.is),o.expect=s=>(i((0,o.is)(s)),s),o.assert=s=>{i((0,o.is)(s))}))(t=e.Arg||={})})(f||={});var g=class{#e;static new(...e){return f.new(...e).toSubpath()}constructor(e){this.#e=e??[]}static is(e){return e instanceof g}toSyscall(){return this.toString()}static fromSyscall(e){return S(e)}components(){return[...this.#e]}isEmpty(){return this.#e.length==0}join(e){return this.#e.push(...e.#e),this}push(e){this.#e.push(e)}pop(){this.#e.pop()}extension(){return this.#e.at(-1)?.split(".").at(-1)}toRelpath(){return f.new(this)}toString(){return this.#e.join("/")}};var N={bundle:async t=>{try{return await syscall("artifact_bundle",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},get:async t=>{try{return await syscall("artifact_get",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},Z={decode:t=>{try{return syscall("base64_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("base64_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},_={bytes:async t=>{try{return await syscall("blob_bytes",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},new:async t=>{try{return await syscall("blob_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},text:async t=>{try{return await syscall("blob_text",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},q={new:async t=>{try{return await syscall("function_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}};var J={new:async t=>{try{return await syscall("resource_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},Q={new:async t=>{try{return await syscall("directory_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},X={new:async t=>{try{return await syscall("file_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},Y={decode:t=>{try{return syscall("hex_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("hex_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},ee=async(t,e)=>{try{return await syscall("include",t,e)}catch(r){throw new Error("The syscall failed.",{cause:r})}},te={decode:t=>{try{return syscall("json_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("json_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},re=t=>{try{return syscall("log",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},F={get:async t=>{try{return await syscall("operation_get",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},run:async t=>{try{return await syscall("operation_run",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},se={new:async t=>{try{return await syscall("command_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},$=t=>{try{return syscall("stack_frame",t+1)}catch(e){throw new Error("The syscall failed.",{cause:e})}},ae={new:async t=>{try{return await syscall("symlink_new",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},ne={decode:t=>{try{return syscall("toml_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("toml_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},oe={decode:t=>{try{return syscall("utf8_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("utf8_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},le={decode:t=>{try{return syscall("yaml_decode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:t=>{try{return syscall("yaml_encode",t)}catch(e){throw new Error("The syscall failed.",{cause:e})}}};var M=async t=>await y.new(t),y=class{#e;#t;#r;#s;static async new(e){let r=await A(e),a,n,o;if(d.Arg.is(r))a=await I(r),n=!1,o=[];else{if(y.is(r))return r;a=await I(r.blob),n=r.executable??!1,o=r.references??[]}return y.fromSyscall(await X.new({blob:a.toSyscall(),executable:n,references:o.map(s=>m.toSyscall(s))}))}constructor(e){this.#e=e.hash,this.#t=e.blob,this.#r=e.executable,this.#s=e.references}static is(e){return e instanceof y}static expect(e){return i(y.is(e)),e}static assert(e){i(y.is(e))}toSyscall(){return{hash:this.#e,blob:this.#t.toSyscall(),executable:this.#r,references:this.#s}}static fromSyscall(e){return new y({hash:e.hash,blob:d.fromSyscall(e.blob),executable:e.executable,references:e.references})}hash(){return this.#e}blob(){return this.#t}executable(){return this.#r}async references(){return await Promise.all(this.#s.map(m.get))}async bytes(){return await this.blob().bytes()}async text(){return await this.blob().text()}};var z=t=>b.new(t),b=class{#e;static new(e){return new b(e)}constructor(e){this.#e=e}static is(e){return e instanceof b}toSyscall(){return{name:this.#e}}static fromSyscall(e){let r=e.name;return new b(r)}name(){return this.#e}};var ie=async t=>await w.new(t),ce=async t=>await(await w.new(t)).download(),w=class{#e;#t;#r;#s;#a;static async new(e){return w.fromSyscall(await J.new({url:e.url,unpack:e.unpack??!1,checksum:e.checksum??void 0,unsafe:e.unsafe??!1}))}constructor(e){this.#e=e.hash,this.#t=e.url,this.#r=e.unpack??!1,this.#s=e.checksum??void 0,this.#a=e.unsafe??!1}static is(e){return e instanceof w}static expect(e){return i(w.is(e)),e}static assert(e){i(w.is(e))}hash(){return this.#e}toSyscall(){return{hash:this.#e,url:this.#t,unpack:this.#r,checksum:this.#s,unsafe:this.#a}}static fromSyscall(e){return new w({hash:e.hash,url:e.url,unpack:e.unpack,checksum:e.checksum,unsafe:e.unsafe})}async download(){let e=await F.run(j.toSyscall(this));return x.fromSyscall(e)}};var U=async(t,...e)=>{let r=[];for(let a=0;a<t.length-1;a++){let n=t[a];r.push(n);let o=e[a];r.push(o)}return r.push(t[t.length-1]),await E(...r)},E=(...t)=>c.new(...t),c=class{#e;static async new(...e){let r=[],a=o=>{if(c.Component.is(o))r.push(o);else if(o instanceof f||o instanceof g)r.push(o.toString());else if(o instanceof c)r.push(...o.components());else if(o instanceof Array)for(let s of o)a(s)};for(let o of await Promise.all(e.map(A)))a(o);let n=[];for(let o of r){let s=n.at(-1);o!==""&&(typeof s=="string"&&typeof o=="string"?n.splice(-1,1,s+o):n.push(o))}return r=n,r=xe(r),new c(r)}constructor(e){this.#e=e}static is(e){return e instanceof c}static expect(e){return i(c.is(e)),e}static assert(e){i(c.is(e))}static async join(e,...r){let a=await E(e),n=await Promise.all(r.map(s=>E(s)));n=n.filter(s=>s.components().length>0);let o=[];for(let s=0;s<n.length;s++){s>0&&o.push(a);let l=n[s];i(l),o.push(l)}return E(...o)}toSyscall(){return{components:this.#e.map(r=>c.Component.toSyscall(r))}}static fromSyscall(e){let r=e.components.map(a=>c.Component.fromSyscall(a));return new c(r)}components(){return[...this.#e]}};(e=>{let t;(o=>(o.is=s=>typeof s=="string"||m.is(s)||s instanceof b,o.toSyscall=s=>typeof s=="string"?{kind:"string",value:s}:m.is(s)?{kind:"artifact",value:m.toSyscall(s)}:s instanceof b?{kind:"placeholder",value:s.toSyscall()}:O(),o.fromSyscall=s=>{switch(s.kind){case"string":return s.value;case"artifact":return m.fromSyscall(s.value);case"placeholder":return b.fromSyscall(s.value);default:return O()}}))(t=e.Component||={})})(c||={});(e=>{let t;(a=>a.is=n=>e.Component.is(n)||n instanceof f||n instanceof g||n instanceof e||n instanceof Array&&n.every(e.Arg.is))(t=e.Arg||={})})(c||={});var we=t=>{let e=t.split(`
`);if(e.length!=1&&(e=e.filter(r=>!/^\s*$/.exec(r)),e=e.map(r=>/^\s*/.exec(r)?.map(n=>n)??[]).flat(),e.length!=0))return e.reduce((r,a)=>{let n=r?.length??0,o=a?.length??0;return n<o?r:a})},xe=t=>{let e;for(let r of t)if(typeof r=="string"){let a=we(r);(a&&!e||a&&e&&a.length<e.length)&&(e=a)}if(e){let r=e;t=t.map(a=>typeof a=="string"?a.split(`
`).map(n=>(n.startsWith(r)&&(n=n.replace(r,"")),n)).join(`
`):a)}return t};var ue=async t=>await h.new(t),h=class{#e;#t;static async new(e){let r=await A(e),a,n;if(typeof r=="string")n=r;else if(f.is(r)||g.is(r))n=r.toString();else if(m.is(r))a=r;else if(r instanceof c){i(r.components().length<=2);let[s,l]=r.components();if(typeof s=="string"&&l===void 0)n=s;else if(m.is(s)&&l===void 0)a=s;else if(m.is(s)&&typeof l=="string")a=s,i(l.startsWith("/")),n=l.slice(1);else throw new Error("Invalid template.")}else{if(r instanceof h)return r;if(typeof r=="object"){a=r.artifact;let s=r.path;typeof s=="string"?n=s:g.is(s)&&(n=s.toString())}}let o;return a!==void 0&&n!==void 0?o=await U`${a}/${n}`:a!==void 0&&n===void 0?o=await U`${a}`:a===void 0&&n!==void 0?o=await U`${n}`:o=await U``,h.fromSyscall(await ae.new({target:o.toSyscall()}))}constructor(e){this.#e=e.hash,this.#t=e.target}static is(e){return e instanceof h}static expect(e){return i(h.is(e)),e}static assert(e){i(h.is(e))}toSyscall(){let e=this.#e,r=this.#t.toSyscall();return{hash:e,target:r}}static fromSyscall(e){let r=e.hash,a=c.fromSyscall(e.target);return new h({hash:r,target:a})}hash(){return this.#e}target(){return this.#t}artifact(){let e=this.#t.components().at(0);if(m.is(e))return e}path(){let[e,r]=this.#t.components();if(typeof e=="string"&&r===void 0)return S(e);if(m.is(e)&&r===void 0)return S();if(m.is(e)&&typeof r=="string")return S(r);throw new Error("Invalid template.")}async resolve(){let e=this;for(;h.is(e);){let r=e.artifact(),a=e.path();if(u.is(r))e=await r.get(a);else if(y.is(r))i(a.components().length===0),e=r;else if(h.is(r))i(a.components().length===0),e=r;else throw new Error("Cannot resolve a symlink without an artifact in its target.")}return e}};var x;(o=>(o.is=s=>s===void 0||typeof s=="boolean"||typeof s=="number"||typeof s=="string"||s instanceof Uint8Array||s instanceof f||s instanceof g||s instanceof d||s instanceof u||s instanceof y||s instanceof h||s instanceof b||s instanceof c||s instanceof v||s instanceof Function||s instanceof w||s instanceof Array||typeof s=="object",o.expect=s=>(i((0,o.is)(s)),s),o.assert=s=>{i((0,o.is)(s))},o.toSyscall=s=>s===void 0?{kind:"null",value:null}:typeof s=="boolean"?{kind:"bool",value:s}:typeof s=="number"?{kind:"number",value:s}:typeof s=="string"?{kind:"string",value:s}:s instanceof Uint8Array?{kind:"bytes",value:s}:s instanceof f?{kind:"relpath",value:s.toSyscall()}:s instanceof g?{kind:"subpath",value:s.toSyscall()}:s instanceof d?{kind:"blob",value:s.toSyscall()}:m.is(s)?{kind:"artifact",value:m.toSyscall(s)}:s instanceof b?{kind:"placeholder",value:s.toSyscall()}:s instanceof c?{kind:"template",value:s.toSyscall()}:j.is(s)?{kind:"operation",value:j.toSyscall(s)}:s instanceof Array?{kind:"array",value:s.map(p=>o.toSyscall(p))}:typeof s=="object"?{kind:"object",value:Object.fromEntries(Object.entries(s).map(([p,P])=>[p,o.toSyscall(P)]))}:O(),o.fromSyscall=s=>{switch(s.kind){case"null":return;case"bool":return s.value;case"number":return s.value;case"string":return s.value;case"bytes":return s.value;case"relpath":return f.fromSyscall(s.value);case"subpath":return g.fromSyscall(s.value);case"blob":return d.fromSyscall(s.value);case"artifact":return m.fromSyscall(s.value);case"placeholder":return b.fromSyscall(s.value);case"template":return c.fromSyscall(s.value);case"operation":return j.fromSyscall(s.value);case"array":return s.value.map(l=>o.fromSyscall(l));case"object":return Object.fromEntries(Object.entries(s.value).map(([l,p])=>[l,o.fromSyscall(p)]));default:return O()}}))(x||={});var me=async t=>await R.new(t),W=async t=>{let e=await R.new(t),r=await F.run(j.toSyscall(e));return x.fromSyscall(r)},pe=async(t,e,r)=>{B.value=Object.fromEntries(Object.entries(e).map(([s,l])=>[s,x.fromSyscall(l)]));let a=r.map(s=>x.fromSyscall(s)),n=await t(...a);return x.toSyscall(n)},R=class extends globalThis.Function{f;hash;packageInstanceHash;modulePath;name;env;args;static async new(e){let r,a,n,o,s,l;if(e instanceof globalThis.Function){r=e;let{module:k,line:C}=$(2);if(i(k.kind==="normal"),a=k.value.packageInstanceHash,n=S(k.value.modulePath),C.startsWith("export default "))o="default";else if(C.startsWith("export let ")){let H=C.match(/^export let ([a-zA-Z0-9]+)\b/)?.at(1);if(!H)throw new Error("Invalid use of tg.function.");o=H}else throw new Error("Invalid use of tg.function.")}else r=e.function.f,a=e.function.packageInstanceHash,n=S(e.function.modulePath),o=e.function.name,s=e.env??{},l=e.args??[];let p=s!==void 0?Object.fromEntries(Object.entries(s).map(([k,C])=>[k,x.toSyscall(C)])):void 0,P=l!==void 0?l.map(k=>x.toSyscall(k)):void 0,T=R.fromSyscall(await q.new({packageInstanceHash:a,modulePath:n.toSyscall(),name:o,env:p,args:P}));return T.f=r,T}constructor(e){return super(),this.f=e.f,this.hash=e.hash,this.packageInstanceHash=e.packageInstanceHash,this.modulePath=S(e.modulePath),this.name=e.name,this.env=e.env,this.args=e.args,new Proxy(this,{apply:async(r,a,n)=>await W({function:r,args:await Promise.all(n.map(A))})})}static is(e){return e instanceof R}static expect(e){return i(R.is(e)),e}static assert(e){i(R.is(e))}toSyscall(){let e=this.hash,r=this.packageInstanceHash,a=this.modulePath.toString(),n=this.name,o=this.env?Object.fromEntries(Object.entries(this.env).map(([l,p])=>[l,x.toSyscall(p)])):void 0,s=this.args?this.args.map(l=>x.toSyscall(l)):void 0;return{hash:e,packageInstanceHash:r,modulePath:a,name:n,env:o,args:s}}static fromSyscall(e){let r=e.hash,a=e.packageInstanceHash,n=e.modulePath,o=e.name,s=e.env!==void 0?Object.fromEntries(Object.entries(e.env).map(([p,P])=>[p,x.fromSyscall(P)])):void 0,l=e.args!==void 0?e.args.map(p=>x.fromSyscall(p)):void 0;return new R({hash:r,packageInstanceHash:a,modulePath:n,name:o,env:s,args:l})}};var j;(a=>(a.is=n=>n instanceof v||n instanceof R||n instanceof w,a.toSyscall=n=>n instanceof v?{kind:"command",value:n.toSyscall()}:n instanceof R?{kind:"function",value:n.toSyscall()}:n instanceof w?{kind:"resource",value:n.toSyscall()}:O(),a.fromSyscall=n=>{switch(n.kind){case"command":return v.fromSyscall(n.value);case"function":return R.fromSyscall(n.value);case"resource":return w.fromSyscall(n.value);default:return O()}}))(j||={});var fe=async t=>await v.new(t),ye=async t=>await(await v.new(t)).run(),he=z("output"),v=class{#e;#t;#r;#s;#a;#n;#o;#l;#i;static async new(e){let r=await A(e),a=r.system,n=await E(r.executable),o=Object.fromEntries(await Promise.all(Object.entries(r.env??{}).map(async([C,H])=>[C,await E(H)]))),s=Object.fromEntries(Object.entries(o).map(([C,H])=>[C,H.toSyscall()])),l=await Promise.all((r.args??[]).map(async C=>(await E(C)).toSyscall())),p=r.checksum??void 0,P=r.unsafe??!1,T=r.network??!1,k=r.hostPaths??[];return v.fromSyscall(await se.new({system:a,executable:n.toSyscall(),env:s,args:l,checksum:p,unsafe:P,network:T,hostPaths:k}))}constructor(e){this.#e=e.hash,this.#t=e.system,this.#r=e.executable,this.#s=e.env,this.#a=e.args,this.#n=e.checksum,this.#o=e.unsafe,this.#l=e.network,this.#i=e.hostPaths}toSyscall(){let e=this.#e,r=this.#t,a=this.#r.toSyscall(),n=Object.fromEntries(Object.entries(this.#s).map(([T,k])=>[T,k.toSyscall()])),o=this.#a.map(T=>T.toSyscall()),s=this.#n,l=this.#o,p=this.#l,P=this.#i;return{hash:e,system:r,executable:a,env:n,args:o,checksum:s,unsafe:l,network:p,hostPaths:P}}static fromSyscall(e){let r=e.hash,a=e.system,n=c.fromSyscall(e.executable),o=Object.fromEntries(Object.entries(e.env).map(([k,C])=>[k,c.fromSyscall(C)])),s=e.args.map(k=>c.fromSyscall(k)),l=e.checksum,p=e.unsafe,P=e.network,T=e.hostPaths;return new v({hash:r,system:a,executable:n,env:o,args:s,checksum:l,unsafe:p,network:P,hostPaths:T})}hash(){return this.#e}async run(){let e=await F.run(j.toSyscall(this));return x.fromSyscall(e)}};var A=async t=>{if(t=await t,t===void 0||typeof t=="boolean"||typeof t=="number"||typeof t=="string"||t instanceof Uint8Array||t instanceof f||t instanceof g||t instanceof d||t instanceof u||t instanceof y||t instanceof h||t instanceof b||t instanceof c||t instanceof v||t instanceof Function||t instanceof w)return t;if(t instanceof Array)return await Promise.all(t.map(e=>A(e)));if(typeof t=="object")return Object.fromEntries(await Promise.all(Object.entries(t).map(async([e,r])=>[e,await A(r)])));throw new Error("Invalid value to resolve.")};var I=async t=>await d.new(t),d=class{#e;static async new(e){let r=await A(e),a;if(r instanceof Uint8Array||typeof r=="string")a=r;else return r;return d.fromSyscall(await _.new(a))}constructor(e){this.#e=e.hash}static is(e){return e instanceof d}static expect(e){return i(d.is(e)),e}static assert(e){i(d.is(e))}toSyscall(){return{hash:this.#e}}static fromSyscall(e){let r=e.hash;return new d({hash:r})}hash(){return this.#e}async bytes(){return await _.bytes(this.toSyscall())}async text(){return await _.text(this.toSyscall())}};(e=>{let t;(o=>(o.is=s=>s instanceof Uint8Array||typeof s=="string"||s instanceof e,o.expect=s=>(i((0,o.is)(s)),s),o.assert=s=>{i((0,o.is)(s))}))(t=e.Arg||={})})(d||={});var de=async(...t)=>await u.new(...t),u=class{#e;#t;static async new(...e){let r={};for(let a of await Promise.all(e.map(A)))if(a!==void 0){if(a instanceof u)for(let[n,o]of Object.entries(await a.entries())){let s=r[n];s instanceof u&&o instanceof u&&(o=await u.new(s,o)),r[n]=o}else if(typeof a=="object")for(let[n,o]of Object.entries(a)){let[s,...l]=S(n).components();if(s===void 0)throw new Error("The path must have at least one component.");let p=s,P=r[p];if(P instanceof u||(P=void 0),l.length>0){let T=S(l).toString(),k=await u.new(P,{[T]:o});r[p]=k}else if(o===void 0)delete r[p];else if(d.Arg.is(o)){let T=await M(o);r[p]=T}else if(y.is(o)||h.is(o))r[p]=o;else{let T=await u.new(P,o);r[p]=T}}}return u.fromSyscall(await Q.new({entries:Object.fromEntries(Object.entries(r).map(([a,n])=>[a,m.toSyscall(n)]))}))}constructor(e){this.#e=e.hash,this.#t=e.entries}static is(e){return e instanceof u}static expect(e){return i(u.is(e)),e}static assert(e){i(u.is(e))}toSyscall(){return{hash:this.#e,entries:this.#t}}static fromSyscall(e){let r=e.hash,a=e.entries;return new u({hash:r,entries:a})}hash(){return this.#e}async get(e){let r=await this.tryGet(e);return i(r,`Failed to get the directory entry "${e}".`),r}async tryGet(e){let r=this;for(let a of S(e).components()){if(!(r instanceof u))return;let n=r.#t[a];if(!n)return;r=await m.get(n)}return r}async entries(){let e={};for await(let[r,a]of this)e[r]=a;return e}async bundle(){let e=m.fromSyscall(await N.bundle(m.toSyscall(this)));return i(u.is(e)),e}async*walk(){for await(let[e,r]of this)if(yield[S(e),r],u.is(r))for await(let[a,n]of r.walk())yield[S(e).join(a),n]}*[Symbol.iterator](){for(let[e,r]of Object.entries(this.#t))yield[e,r]}async*[Symbol.asyncIterator](){for(let e of Object.keys(this.#t))yield[e,await this.get(e)]}};var m;(s=>(s.is=l=>l instanceof u||l instanceof y||l instanceof h,s.expect=l=>(i((0,s.is)(l)),l),s.assert=l=>{i((0,s.is)(l))},s.get=async l=>s.fromSyscall(await N.get(l)),s.toSyscall=l=>l instanceof u?{kind:"directory",value:l.toSyscall()}:l instanceof y?{kind:"file",value:l.toSyscall()}:l instanceof h?{kind:"symlink",value:l.toSyscall()}:O(),s.fromSyscall=l=>{switch(l.kind){case"directory":return u.fromSyscall(l.value);case"file":return y.fromSyscall(l.value);case"symlink":return h.fromSyscall(l.value);default:return O()}}))(m||={});var ge=(t,e)=>({callSites:e.map(a=>({typeName:a.getTypeName(),functionName:a.getFunctionName(),methodName:a.getMethodName(),fileName:a.getFileName(),lineNumber:a.getLineNumber(),columnNumber:a.getColumnNumber(),isEval:a.isEval(),isNative:a.isNative(),isConstructor:a.isConstructor(),isAsync:a.isAsync(),isPromiseAll:a.isPromiseAll(),promiseIndex:a.getPromiseIndex()}))});var be=async t=>{let e=$(1);return m.fromSyscall(await ee(e,t))};var K=(...t)=>{let e=t.map(r=>ke(r)).join(" ");re(e)},ke=t=>D(t,new WeakSet),D=(t,e)=>{switch(typeof t){case"string":return`"${t}"`;case"number":return t.toString();case"boolean":return t?"true":"false";case"undefined":return"undefined";case"object":return t===null?"null":Se(t,e);case"function":return`(function "${t.name??"(anonymous)"}")`;case"symbol":return"(symbol)";case"bigint":return t.toString()}},Se=(t,e)=>{if(e.has(t))return"(circular)";if(e.add(t),t instanceof Array)return`[${t.map(r=>D(r,e)).join(", ")}]`;if(t instanceof Error)return t.stack??"";if(t instanceof Promise)return"(promise)";if(t instanceof u)return`(tg.directory ${t.hash()})`;if(t instanceof y)return`(tg.file ${t.hash()})`;if(t instanceof h)return`(tg.symlink ${t.hash()})`;if(t instanceof b)return`(tg.placeholder "${t.name()}")`;if(t instanceof c)return`(tg.template "${t.components().map(a=>typeof a=="string"?a:`\${${D(a,e)}}`).join("")}")`;{let r="";t.constructor!==void 0&&t.constructor.name!=="Object"&&(r=`${t.constructor.name} `);let a=Object.entries(t).map(([n,o])=>`${n}: ${D(o,e)}`);return`${r}{ ${a.join(", ")} }`}};var Ae=t=>{if(typeof t=="string")return t;{let{arch:e,os:r}=t;return`${e}_${r}`}},L;(a=>(a.is=n=>n==="amd64_linux"||n==="arm64_linux"||n==="amd64_macos"||n==="arm64_macos",a.arch=n=>{switch(n){case"amd64_linux":case"amd64_macos":return"amd64";case"arm64_linux":case"arm64_macos":return"arm64";default:throw new Error("Invalid system.")}},a.os=n=>{switch(n){case"amd64_linux":case"arm64_linux":return"linux";case"amd64_macos":case"arm64_macos":return"macos";default:throw new Error("Invalid system.")}}))(L||={});Object.defineProperties(Error,{prepareStackTrace:{value:ge}});var Pe={log:K};Object.defineProperties(globalThis,{console:{value:Pe}});var Te={Artifact:m,Blob:d,Directory:u,File:y,Function:R,Placeholder:b,Relpath:f,Subpath:g,Symlink:h,System:L,Template:c,Value:x,base64:Z,blob:I,call:W,command:fe,directory:de,download:ce,entrypoint:pe,env:B,file:M,function:me,hex:Y,include:be,json:te,log:K,output:he,placeholder:z,relpath:G,resolve:A,resource:ie,run:ye,subpath:S,symlink:ue,system:Ae,template:E,toml:ne,utf8:oe,yaml:le};Object.defineProperties(globalThis,{tg:{value:Te},t:{value:U}});})();
//# sourceMappingURL=global.js.map
