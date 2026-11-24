// Positions and radii are prepared in frame pixels. Colors arrive premultiplied
// in the interpolation space. The attachment performs linear source-over.
struct Params {
    geom: vec4f, // origin and axis/radii
    misc: vec4f, // inner fraction, start degrees, angular span, spare
    mode: vec4u, // geometry, extend, space, stop count
    options: vec4u, // dither, workload, width, height
}
struct Stop { color: vec4f, data: vec4f }
@group(0) @binding(0) var<uniform> p: Params;
@group(0) @binding(1) var<storage, read> stops: array<Stop>;
@group(0) @binding(2) var strip: texture_2d<f32>;
@group(0) @binding(3) var filtering: sampler;
@group(0) @binding(4) var<storage, read> lut:array<vec2f>;
fn transfer(x:f32, ch:u32)->f32 {
    let t=clamp(x,0.,1.)*4096.; let i=u32(t); return mix(lut[i][ch],lut[min(i+1u,4096u)][ch],fract(t));
}
fn table(v:vec3f,ch:u32)->vec3f {return vec3f(transfer(v.r,ch),transfer(v.g,ch),transfer(v.b,ch));}
@vertex fn vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    return vec4f(array(vec2f(-1,-1), vec2f(3,-1), vec2f(-1,3))[i],0,1);
}
fn encode(x: vec3f) -> vec3f {
    return select(1.055 * pow(x,vec3f(1./2.4)) - .055, 12.92*x, x <= vec3f(.0031308));
}
fn decode(x: vec3f) -> vec3f {
    return select(pow((x+.055)/1.055,vec3f(2.4)), x/12.92, x <= vec3f(.04045));
}
fn rgb(v:vec3f) -> vec3f {
    let lms = vec3f(dot(v,vec3f(1,.3963377774,.2158037573)),
                    dot(v,vec3f(1,-.1055613458,-.0638541728)),
                    dot(v,vec3f(1,-.0894841775,-1.291485548)));
    let q=lms*lms*lms;
    return clamp(vec3f(dot(q,vec3f(4.0767416621,-3.3077115913,.2309699292)),
                       dot(q,vec3f(-1.2684380046,2.6097574011,-.3413193965)),
                       dot(q,vec3f(-.0041960863,-.7034186147,1.707614701))),vec3f(0),vec3f(1));
}
fn bayer(xy:vec2u) -> f32 {
    var rank=0u;
    for (var bit=0u;bit<3u;bit++) {
        let x=(xy.x>>bit)&1u; let y=(xy.y>>bit)&1u;
        rank=4u*rank+2u*(x^y)+y;
    }
    return ((f32(rank)+.5)/64.-.5)/255.;
}
fn sample_ramp(t0:f32) -> vec4f {
    var t=clamp(t0,0.,1.);
    if p.mode.y==1u { t=fract(t0); }
    if p.mode.y==2u { t=1.-abs(1.-2.*fract(.5*t0)); }
    var a=stops[0];
    for(var i=1u;i<p.mode.w;i++) {
        let b=stops[i];
        if t < b.data.x {
            if t<=a.data.x {break;}
            a.color=mix(a.color,b.color,(t-a.data.x)/(b.data.x-a.data.x));
            break;
        }
        a=b;
    }
    if a.color.a==0. {return vec4f(0);}
    var c=a.color.rgb/a.color.a;
    if p.mode.z==1u { c=decode(c); }
    if p.mode.z==2u { c=rgb(c); }
    return vec4f(c,a.color.a);
}
@fragment fn fragment(@builtin(position) pixel:vec4f) -> @location(0) vec4f {
    let d=pixel.xy-p.geom.xy;
    var t=dot(d,p.geom.zw)/dot(p.geom.zw,p.geom.zw);
    if p.mode.x==1u { t=(length(d/p.geom.zw)-p.misc.x)/(1.-p.misc.x); }
    if p.mode.x==2u {
        var a=0.;
        if any(d!=vec2f(0)) {
            a=atan2(d.y,d.x)*57.295779513-p.misc.y;
            a=a-360.*floor(a/360.);
        }
        t=a/p.misc.z;
    }
    // A reciprocal multiply may put an exact integer just below the seam.
    // Evaluate the residual relative to the nearest period's origin.
    if p.mode.x==0u && p.mode.y==1u {
        let period=round(t);
        t=dot(d-period*p.geom.zw,p.geom.zw)/dot(p.geom.zw,p.geom.zw);
    }
    let work=p.options.y;
    if work==4u {
        // Full turn with a hard stop halfway. The two discontinuities lie
        // on opposite rays. misc.zw is the prepared unit starting ray.
        let side=p.misc.z*d.y-p.misc.w*d.x;
        let end=side<0. || (side==0. && dot(d,p.misc.zw)<0.);
        return vec4f(vec3f(select(0.,1.,end)),1.);
    }
    if work==1u || work==2u {
        t=1.-(pixel.y-p.geom.y)/p.geom.w;
    }
    if work==2u {
        // Map interval endpoints to first/last texel centers.
        return textureSampleLevel(strip,filtering,vec2f((.5+255.*clamp(t,0.,1.))/256.,.5),0.);
    }
    var c=sample_ramp(t);
    if p.options.x==2u && c.a>0. {
        c=vec4f(table(clamp(table(c.rgb,0u)+bayer(vec2u(pixel.xy)),vec3f(0),vec3f(1)),1u),c.a);
    } else if p.options.x!=0u && c.a>0. {
        c=vec4f(decode(clamp(encode(c.rgb)+bayer(vec2u(pixel.xy)),vec3f(0),vec3f(1))),c.a);
    }
    if work==3u {
        // Straight section of an analytic border over the gradient interior.
        let border=clamp(2.75-pixel.y,0.,1.)*.6;
        let alpha=border+c.a*(1.-border);
        return vec4f((border*vec3f(.1,.2,.5)+(1.-border)*c.rgb*c.a)/alpha,alpha);
    }
    return c;
}
