"use strict";(()=>{var A=(s,e)=>{if(!s)throw new Error(e??"Failed assertion.")},P=s=>{throw new Error(s??"Reached unreachable code.")};var I={bundle:async s=>{try{return await syscall("artifact_bundle",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}},get:async s=>{try{return await syscall("artifact_get",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},q={decode:s=>{try{return syscall("base64_decode",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:s=>{try{return syscall("base64_encode",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},B={bytes:async s=>{try{return await syscall("blob_bytes",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}},new:async s=>{try{return await syscall("blob_new",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}},text:async s=>{try{return await syscall("blob_text",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},G={new:async(s,e,t)=>{try{return await syscall("call_new",s,e,t)}catch(a){throw new Error("The syscall failed.",{cause:a})}}};var Z={new:async(s,e,t,a)=>{try{return await syscall("download_new",s,e,t,a)}catch(r){throw new Error("The syscall failed.",{cause:r})}}},J={new:async s=>{try{return await syscall("directory_new",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},Q={new:async(s,e,t)=>{try{return await syscall("file_new",s,e,t)}catch(a){throw new Error("The syscall failed.",{cause:a})}}},X={decode:s=>{try{return syscall("hex_decode",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:s=>{try{return syscall("hex_encode",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},Y=async(s,e)=>{try{return await syscall("include",s,e)}catch(t){throw new Error("The syscall failed.",{cause:t})}},ee={decode:s=>{try{return syscall("json_decode",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:s=>{try{return syscall("json_encode",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},te=s=>{try{return syscall("log",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}},j={get:async s=>{try{return await syscall("operation_get",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}},run:async s=>{try{return await syscall("operation_run",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},re={new:async(s,e,t,a,r,n,l,u)=>{try{return await syscall("process_new",s,e,t,a,r,n,l,u)}catch(b){throw new Error("The syscall failed.",{cause:b})}}},M=s=>{try{return syscall("stack_frame",s+1)}catch(e){throw new Error("The syscall failed.",{cause:e})}},se={new:async s=>{try{return await syscall("symlink_new",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},ae={decode:s=>{try{return syscall("toml_decode",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:s=>{try{return syscall("toml_encode",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},ne={decode:s=>{try{return syscall("utf8_decode",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:s=>{try{return syscall("utf8_encode",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}}},le={decode:s=>{try{return syscall("yaml_decode",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}},encode:s=>{try{return syscall("yaml_encode",s)}catch(e){throw new Error("The syscall failed.",{cause:e})}}};var y=class{#e;#t;#r;#s;static async new(e){let t=await w(e),a,r,n;if(p.Arg.is(t))a=await F(t),r=!1,n=[];else{if(y.is(t))return t;a=await F(t.blob),r=t.executable??!1,n=t.references??[]}return y.fromSyscall(await Q.new(a.toSyscall(),r,n.map(l=>i.toSyscall(l))))}constructor(e){this.#e=e.hash,this.#t=e.blob,this.#r=e.executable,this.#s=e.references}static is(e){return e instanceof y}toSyscall(){return{hash:this.#e,blob:this.#t.toSyscall(),executable:this.#r,references:this.#s}}static fromSyscall(e){return new y({hash:e.hash,blob:p.fromSyscall(e.blob),executable:e.executable,references:e.references})}hash(){return this.#e}blob(){return this.#t}executable(){return this.#r}async references(){return await Promise.all(this.#s.map(i.get))}async bytes(){return await this.blob().bytes()}async text(){return await this.blob().text()}},_=y.new;var m=class{#e;static new(...e){let t=[],a=n=>{if(typeof n=="string")for(let l of n.split("/"))l===""||l==="."||(l===".."?t.push({kind:"parent"}):t.push({kind:"normal",value:l}));else if(m.Component.is(n))t.push(n);else if(n instanceof m)t.push(...n.components());else if(n instanceof Array)for(let l of n)a(l)};for(let n of e)a(n);let r=new m;for(let n of t)r.push(n);return r}constructor(e=[]){this.#e=e}static is(e){return e instanceof m}toSyscall(){return this.toString()}static fromSyscall(e){return h(e)}components(){return[...this.#e]}push(e){if(e.kind==="parent"){let t=this.#e.at(-1);t===void 0||t.kind==="parent"?this.#e.push(e):this.#e.pop()}else this.#e.push(e)}join(e){let t=h(this);for(let a of h(e).components())t.push(a);return t}diff(e){let t=h(e),a=h(this);for(;;){let n=t.#e.at(0),l=a.#e.at(0);if(n&&l&&m.Component.equal(n,l))t.#e.shift(),a.#e.shift();else break}if(t.#e.at(0)?.kind==="parent")throw new Error(`There is no valid path from "${t}" to "${a}".`);return h(Array.from({length:t.#e.length},()=>({kind:"parent"})),a)}toString(){return this.#e.map(e=>{switch(e.kind){case"parent":return"..";case"normal":return e.value}}).join("/")}};(e=>{let s;(r=>(r.is=n=>typeof n=="object"&&n!==null&&"kind"in n&&(n.kind==="parent"||n.kind==="normal"),r.equal=(n,l)=>n.kind===l.kind&&(n.kind==="normal"&&l.kind==="normal"?n.value===l.value:!0)))(s=e.Component||={})})(m||={});(e=>{let s;(a=>a.is=r=>typeof r=="string"||e.Component.is(r)||r instanceof e||r instanceof Array&&r.every(e.Arg.is))(s=e.Arg||={})})(m||={});var h=m.new;var d=class{#e;static new(e){return new d(e)}constructor(e){this.#e=e}static is(e){return e instanceof d}toSyscall(){return{name:this.#e}}static fromSyscall(e){let t=e.name;return new d(t)}name(){return this.#e}},N=d.new;var U=async(s,...e)=>{let t=[];for(let a=0;a<s.length-1;a++){let r=s[a];t.push(r);let n=e[a];t.push(n)}return t.push(s[s.length-1]),await O(...t)},o=class{#e;static async new(...e){let t=[],a=n=>{if(o.Component.is(n))t.push(n);else if(n instanceof m)t.push(n.toString());else if(n instanceof o)t.push(...n.components());else if(n instanceof Array)for(let l of n)a(l)};for(let n of await Promise.all(e.map(w)))a(n);let r=[];for(let n of t){let l=r.at(-1);n!==""&&(typeof l=="string"&&typeof n=="string"?r.splice(-1,1,l+n):r.push(n))}return t=r,new o(t)}constructor(e){this.#e=e}static is(e){return e instanceof o}static async join(e,...t){let a=await O(e),r=await Promise.all(t.map(l=>O(l)));r=r.filter(l=>l.components().length>0);let n=[];for(let l=0;l<r.length;l++){l>0&&n.push(a);let u=r[l];A(u),n.push(u)}return O(...n)}toSyscall(){return{components:this.#e.map(t=>o.Component.toSyscall(t))}}static fromSyscall(e){let t=e.components.map(a=>o.Component.fromSyscall(a));return new o(t)}components(){return[...this.#e]}};(e=>{let s;(n=>(n.is=l=>typeof l=="string"||i.is(l)||l instanceof d,n.toSyscall=l=>typeof l=="string"?{kind:"string",value:l}:i.is(l)?{kind:"artifact",value:i.toSyscall(l)}:l instanceof d?{kind:"placeholder",value:l.toSyscall()}:P(),n.fromSyscall=l=>{switch(l.kind){case"string":return l.value;case"artifact":return i.fromSyscall(l.value);case"placeholder":return d.fromSyscall(l.value);default:return P()}}))(s=e.Component||={})})(o||={});(e=>{let s;(a=>a.is=r=>e.Component.is(r)||r instanceof m||r instanceof e||r instanceof Array&&r.every(e.Arg.is))(s=e.Arg||={})})(o||={});var O=o.new;var f=class{#e;#t;static async new(e){let t=await w(e),a,r;if(typeof t=="string")r=t;else if(m.is(t))r=t.toString();else if(i.is(t))a=t;else if(t instanceof o){A(t.components().length<=2);let[l,u]=t.components();if(typeof l=="string"&&u===void 0)r=l;else if(i.is(l)&&u===void 0)a=l;else if(i.is(l)&&typeof u=="string")a=l,A(u.startsWith("/")),r=u.slice(1);else throw new Error("Invalid template.")}else{if(t instanceof f)return t;if(typeof t=="object"){a=t.artifact;let l=t.path;typeof l=="string"?r=l:m.is(l)&&(r=l.toString())}}let n;return a!==void 0&&r!==void 0?n=await U`${a}/${r}`:a!==void 0&&r===void 0?n=await U`${a}`:a===void 0&&r!==void 0?n=await U`${r}`:n=await U``,f.fromSyscall(await se.new(n.toSyscall()))}constructor(e){this.#e=e.hash,this.#t=e.target}static is(e){return e instanceof f}toSyscall(){let e=this.#e,t=this.#t.toSyscall();return{hash:e,target:t}}static fromSyscall(e){let t=e.hash,a=o.fromSyscall(e.target);return new f({hash:t,target:a})}hash(){return this.#e}target(){return this.#t}artifact(){let e=this.#t.components().at(0);if(i.is(e))return e}path(){let[e,t]=this.#t.components();if(typeof e=="string"&&t===void 0)return h(e);if(i.is(e)&&t===void 0)return h();if(i.is(e)&&typeof t=="string")return h(t);throw new Error("Invalid template.")}async resolve(){let e=this;for(;f.is(e);){let t=e.artifact(),a=e.path();if(c.is(t))e=await t.get(a);else if(y.is(t))A(a.components().length===0),e=t;else if(f.is(t))A(a.components().length===0),e=t;else throw new Error("Cannot resolve a symlink without an artifact in its target.")}return e}},oe=f.new;var w=async s=>{if(s=await s,s==null||typeof s=="boolean"||typeof s=="number"||typeof s=="string"||s instanceof Uint8Array||s instanceof m||s instanceof p||s instanceof c||s instanceof y||s instanceof f||s instanceof d||s instanceof o)return s;if(s instanceof Array)return await Promise.all(s.map(e=>w(e)));if(typeof s=="object")return Object.fromEntries(await Promise.all(Object.entries(s).map(async([e,t])=>[e,await w(t)])));throw new Error("Invalid value to resolve.")};var p=class{#e;static async new(e){let t=await w(e),a;if(t instanceof Uint8Array||typeof t=="string")a=t;else return t;return p.fromSyscall(await B.new(a))}constructor(e){this.#e=e.hash}static is(e){return e instanceof p}toSyscall(){return{hash:this.#e}}static fromSyscall(e){let t=e.hash;return new p({hash:t})}hash(){return this.#e}async bytes(){return await B.bytes(this.toSyscall())}async text(){return await B.text(this.toSyscall())}};(e=>{let s;(a=>a.is=r=>r instanceof Uint8Array||typeof r=="string"||r instanceof e)(s=e.Arg||={})})(p||={});var F=p.new;var g;(a=>(a.is=r=>r==null||typeof r=="boolean"||typeof r=="number"||typeof r=="string"||r instanceof Uint8Array||r instanceof m||r instanceof p||r instanceof c||r instanceof y||r instanceof f||r instanceof d||r instanceof o||r instanceof Array||typeof r=="object",a.toSyscall=r=>r==null?{kind:"null",value:r}:typeof r=="boolean"?{kind:"bool",value:r}:typeof r=="number"?{kind:"number",value:r}:typeof r=="string"?{kind:"string",value:r}:r instanceof Uint8Array?{kind:"bytes",value:r}:r instanceof m?{kind:"path",value:r.toSyscall()}:r instanceof p?{kind:"blob",value:r.toSyscall()}:i.is(r)?{kind:"artifact",value:i.toSyscall(r)}:r instanceof d?{kind:"placeholder",value:r.toSyscall()}:r instanceof o?{kind:"template",value:r.toSyscall()}:r instanceof Array?{kind:"array",value:r.map(l=>a.toSyscall(l))}:typeof r=="object"?{kind:"object",value:Object.fromEntries(Object.entries(r).map(([l,u])=>[l,a.toSyscall(u)]))}:P(),a.fromSyscall=r=>{switch(r.kind){case"null":return r.value;case"bool":return r.value;case"number":return r.value;case"string":return r.value;case"bytes":return r.value;case"path":return m.fromSyscall(r.value);case"blob":return p.fromSyscall(r.value);case"artifact":return i.fromSyscall(r.value);case"placeholder":return d.fromSyscall(r.value);case"template":return o.fromSyscall(r.value);case"array":return r.value.map(n=>a.fromSyscall(n));case"object":return Object.fromEntries(Object.entries(r.value).map(([n,l])=>[n,a.fromSyscall(l)]));default:return P()}}))(g||={});var R;(e=>e.is=t=>t==null)(R||={});var c=class{#e;#t;static async new(...e){let t=new Map;for(let a of await Promise.all(e.map(w)))if(!R.is(a)){if(a instanceof c)for(let[r,n]of await a.entries()){let l=t.get(r);l instanceof c&&n instanceof c&&(n=await c.new(l,n)),t.set(r,n)}else if(typeof a=="object")for(let[r,n]of Object.entries(a)){let[l,...u]=h(r).components();if(l===void 0)throw new Error("The path must have at least one component.");if(l.kind!=="normal")throw new Error("Invalid path component.");let b=l.value,V=t.get(b);if(V instanceof c||(V=void 0),u.length>0){let x=h(u).toString(),k=await c.new(V,{[x]:n});t.set(b,k)}else if(R.is(n))t.delete(b);else if(p.Arg.is(n)){let x=await _(n);t.set(b,x)}else if(y.is(n)||f.is(n))t.set(b,n);else{let x=await c.new(V,n);t.set(b,x)}}}return c.fromSyscall(await J.new(new Map(Array.from(t,([a,r])=>[a,i.toSyscall(r)]))))}constructor(e){this.#e=e.hash,this.#t=e.entries}static is(e){return e instanceof c}toSyscall(){return{hash:this.#e,entries:Object.fromEntries(this.#t)}}static fromSyscall(e){let t=e.hash,a=new Map(Object.entries(e.entries));return new c({hash:t,entries:a})}hash(){return this.#e}async get(e){let t=await this.tryGet(e);return A(t,`Failed to get the directory entry "${e}".`),t}async tryGet(e){let t=this;for(let a of h(e).components()){if(A(a.kind==="normal"),!(t instanceof c))return;let r=t.#t.get(a.value);if(!r)return;t=await i.get(r)}return t}async entries(){let e=new Map;for await(let[t,a]of this)e.set(t,a);return e}async bundle(){let e=i.fromSyscall(await I.bundle(i.toSyscall(this)));return A(c.is(e)),e}async*walk(){for await(let[e,t]of this)if(yield[h(e),t],c.is(t))for await(let[a,r]of t.walk())yield[h(e).join(a),r]}*[Symbol.iterator](){for(let[e,t]of this.#t)yield[e,t]}async*[Symbol.asyncIterator](){for(let e of this.#t.keys())yield[e,await this.get(e)]}},ie=c.new;var i;(r=>(r.is=n=>n instanceof c||n instanceof y||n instanceof f,r.get=async n=>r.fromSyscall(await I.get(n)),r.toSyscall=n=>n instanceof c?{kind:"directory",value:n.toSyscall()}:n instanceof y?{kind:"file",value:n.toSyscall()}:n instanceof f?{kind:"symlink",value:n.toSyscall()}:P(),r.fromSyscall=n=>{switch(n.kind){case"directory":return c.fromSyscall(n.value);case"file":return y.fromSyscall(n.value);case"symlink":return f.fromSyscall(n.value);default:return P()}}))(i||={});var $=new Map;var T=class extends globalThis.Function{packageInstanceHash;modulePath;name;f;static new(e){let{module:t,line:a}=M(1);A(t.kind==="normal");let r=t.value.packageInstanceHash,n=t.value.modulePath,l;if(a.startsWith("export default "))l="default";else if(a.startsWith("export let ")){let u=a.match(/^export let ([a-zA-Z0-9]+)\b/)?.at(1);if(!u)throw new Error("Invalid use of tg.function.");l=u}else throw new Error("Invalid use of tg.function.");return new T({packageInstanceHash:r,modulePath:n,name:l,f:e})}constructor(e){return super(),this.packageInstanceHash=e.packageInstanceHash,this.modulePath=h(e.modulePath),this.name=e.name,this.f=e.f,new Proxy(this,{apply:async(t,a,r)=>{let n=await Promise.all(r.map(w));return await D({function:t,args:n})}})}static is(e){return e instanceof T}toSyscall(){let e=this.packageInstanceHash,t=this.modulePath.toString(),a=this.name;return{packageInstanceHash:e,modulePath:t,name:a}}static fromSyscall(e){let t=e.packageInstanceHash,a=e.modulePath,r=e.name;return new T({packageInstanceHash:t,modulePath:a,name:r})}async run(e,t){for(let[n,l]of Object.entries(e))$.set(n,g.fromSyscall(l));let a=t.map(g.fromSyscall);A(this.f);let r=await this.f(...a);return g.toSyscall(r)}},ce=T.new;var ue=async s=>await(await v.new(s)).run(),v=class{#e;#t;#r;#s;#a;static async new(e){return v.fromSyscall(await Z.new(e.url,e.unpack??!1,e.checksum??null,e.unsafe??!1))}constructor(e){this.#e=e.hash,this.#t=e.url,this.#r=e.unpack??!1,this.#s=e.checksum??null,this.#a=e.unsafe??!1}static is(e){return e instanceof v}hash(){return this.#e}toSyscall(){return{hash:this.#e,url:this.#t,unpack:this.#r,checksum:this.#s,unsafe:this.#a}}static fromSyscall(e){return new v({hash:e.hash,url:e.url,unpack:e.unpack,checksum:e.checksum,unsafe:e.unsafe})}async run(){let e=await j.run(E.toSyscall(this));return g.fromSyscall(e)}};var me=async s=>await(await H.new(s)).run(),ye=N("output"),H=class{#e;#t;#r;#s;#a;#n;#l;#o;#i;static async new(e){let t=await w(e),a=t.system,r=await O(t.executable),n=Object.fromEntries(await Promise.all(Object.entries(t.env??{}).map(async([k,z])=>[k,await O(z)]))),l=await Promise.all((t.args??[]).map(async k=>await O(k))),u=t.checksum??null,b=t.unsafe??!1,V=t.network??!1,x=t.hostPaths??[];return H.fromSyscall(await re.new(a,r.toSyscall(),n,l.map(k=>k.toSyscall()),u,b,V,x))}constructor(e){this.#e=e.hash,this.#t=e.system,this.#r=e.executable,this.#s=e.env,this.#a=e.args,this.#n=e.checksum,this.#l=e.unsafe,this.#o=e.network,this.#i=e.hostPaths}hash(){return this.#e}toSyscall(){let e=this.#e,t=this.#t,a=this.#r.toSyscall(),r=Object.fromEntries(Object.entries(this.#s).map(([x,k])=>[x,k.toSyscall()])),n=this.#a.map(x=>x.toSyscall()),l=this.#n,u=this.#l,b=this.#o,V=this.#i;return{hash:e,system:t,executable:a,env:r,args:n,checksum:l,unsafe:u,network:b,hostPaths:V}}static fromSyscall(e){let t=e.hash,a=e.system,r=o.fromSyscall(e.executable),n=Object.fromEntries(Object.entries(e.env).map(([k,z])=>[k,o.fromSyscall(z)])),l=e.args.map(k=>o.fromSyscall(k)),u=e.checksum,b=e.unsafe,V=e.network,x=e.hostPaths;return new H({hash:t,system:a,executable:r,env:n,args:l,checksum:u,unsafe:b,network:V,hostPaths:x})}async run(){let e=await j.run(E.toSyscall(this));return g.fromSyscall(e)}};var E;(a=>(a.is=r=>r instanceof C||r instanceof v||r instanceof H,a.toSyscall=r=>r instanceof v?{kind:"download",value:r.toSyscall()}:r instanceof H?{kind:"process",value:r.toSyscall()}:r instanceof C?{kind:"call",value:r.toSyscall()}:P(),a.fromSyscall=(r,n)=>{switch(n.kind){case"download":return v.fromSyscall(n.value);case"process":return H.fromSyscall(n.value);case"call":return C.fromSyscall(n.value);default:return P()}}))(E||={});var D=async s=>await(await C.new(s)).run(),C=class{#e;#t;#r;#s;static async new(e){let t=e.function.toSyscall(),a=Object.fromEntries(Object.entries(e.env??{}).map(([l,u])=>[l,g.toSyscall(u)])),r=(e.args??[]).map(l=>g.toSyscall(l));return C.fromSyscall(await G.new(t,a,r))}constructor(e){this.#e=e.hash,this.#t=e.function,this.#r=e.env,this.#s=e.args}static is(e){return e instanceof C}hash(){return this.#e}toSyscall(){let e=this.#e,t=this.#t.toSyscall(),a=Object.fromEntries(Array.from(this.#r.entries()).map(([n,l])=>[n,g.toSyscall(l)])),r=this.#s.map(n=>g.toSyscall(n));return{hash:e,function:t,env:a,args:r}}static fromSyscall(e){let t=e.hash,a=T.fromSyscall(e.function),r=new Map(Object.entries(e.env).map(([l,u])=>[l,g.fromSyscall(u)])),n=e.args.map(l=>g.fromSyscall(l));return new C({hash:t,function:a,env:r,args:n})}async run(){let e=await j.run(E.toSyscall(this));return g.fromSyscall(e)}};var pe=(s,e)=>({callSites:e.map(a=>({typeName:a.getTypeName(),functionName:a.getFunctionName(),methodName:a.getMethodName(),fileName:a.getFileName(),lineNumber:a.getLineNumber(),columnNumber:a.getColumnNumber(),isEval:a.isEval(),isNative:a.isNative(),isConstructor:a.isConstructor(),isAsync:a.isAsync(),isPromiseAll:a.isPromiseAll(),promiseIndex:a.getPromiseIndex()}))});var fe=async s=>{let e=M(1);return i.fromSyscall(await Y(e,s))};var L=(...s)=>{let e=s.map(t=>de(t)).join(" ");te(e)},de=s=>K(s,new Set),K=(s,e)=>{switch(typeof s){case"string":return`"${s}"`;case"number":return s.toString();case"boolean":return s?"true":"false";case"undefined":return"undefined";case"object":return ge(s,e);case"function":return`[function ${s.name??"(anonymous)"}]`;case"symbol":return"[symbol]";case"bigint":return s.toString()}},ge=(s,e)=>{if(s===null)return"null";if(e.has(s))return"[circular]";if(e.add(s),s instanceof Array)return`[${s.map(t=>K(t,e)).join(", ")}]`;if(s instanceof Error)return s.stack??"";if(s instanceof Promise)return"[promise]";{let t="";s.constructor!==void 0&&s.constructor.name!=="Object"&&(t=`${s.constructor.name} `);let a=Object.entries(s).map(([r,n])=>`${r}: ${K(n,e)}`);return`${t}{ ${a.join(", ")} }`}};var he=s=>{if(typeof s=="string")return s;{let{arch:e,os:t}=s;return`${e}_${t}`}},W;(t=>(t.arch=a=>{switch(a){case"amd64_linux":case"amd64_macos":return"amd64";case"arm64_linux":case"arm64_macos":return"arm64";default:throw new Error("Invalid system.")}},t.os=a=>{switch(a){case"amd64_linux":case"arm64_linux":return"linux";case"amd64_macos":case"arm64_macos":return"macos";default:throw new Error("Invalid system.")}}))(W||={});Object.defineProperties(Error,{prepareStackTrace:{value:pe}});var we={log:L};Object.defineProperties(globalThis,{console:{value:we}});var be={Artifact:i,Blob:p,Directory:c,File:y,Function:T,Path:m,Placeholder:d,Symlink:f,System:W,Template:o,Value:g,base64:q,blob:F,call:D,directory:ie,download:ue,env:$,file:_,function:ce,hex:X,include:fe,json:ee,log:L,nullish:R,output:ye,path:h,placeholder:N,process:me,resolve:w,symlink:oe,toml:ae,system:he,template:O,utf8:ne,yaml:le};Object.defineProperties(globalThis,{tg:{value:be},t:{value:U}});})();
//# sourceMappingURL=global.js.map
