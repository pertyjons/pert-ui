// shares the analytic coverage function and the shape records at group 0.
struct GradientStop {
    components: vec4f,
    exact: vec4f,
    offset: vec4f,
}
struct GradientRecord {
    coefficients: vec4f,
    settings: vec4u, // count, space, extend, inner fraction (-1 for linear)
    stops: array<GradientStop,16>,
}
@group(1) @binding(0) var<storage,read> gradients: array<GradientRecord>;

fn encoded_rgb(c:vec3f)->vec3f {
    return select(1.055*pow(c,vec3f(1./2.4))-.055,12.92*c,c<=vec3f(.0031308));
}
fn decoded_rgb(c:vec3f)->vec3f {
    return select(pow((c+.055)/1.055,vec3f(2.4)), c/12.92,c<=vec3f(.04045));
}
fn linear_from_lab(v:vec3f)->vec3f {
    let lms=vec3f(dot(v,vec3f(1,.3963377774,.2158037573)),
                  dot(v,vec3f(1,-.1055613458,-.0638541728)),
                  dot(v,vec3f(1,-.0894841775,-1.291485548)));
    let cube=lms*lms*lms;
    return clamp(vec3f(dot(cube,vec3f(4.0767416621,-3.3077115913,.2309699292)),
                       dot(cube,vec3f(-1.2684380046,2.6097574011,-.3413193965)),
                       dot(cube,vec3f(-.0041960863,-.7034186147,1.707614701))),vec3f(0),vec3f(1));
}
fn gradient_color(index:u32,pixel:vec2f)->vec4f {
    let coeff=gradients[index].coefficients;
    let settings=gradients[index].settings;
    let inner=bitcast<f32>(settings.w);
    let delta=(pixel-coeff.xy)*coeff.zw;
    var t=delta.x+delta.y;
    if inner>=0. { t=(length(delta)-inner)/(1.-inner); }
    switch settings.z {
        case 1u:{t=fract(t);}
        case 2u:{t=1.-abs(1.-2.*fract(t*.5));}
        default:{t=clamp(t,0.,1.);}
    }
    var lower=0u;
    for(var i=1u;i<settings.x;i++){
        if gradients[index].stops[i].offset.x>t {break;}
        lower=i;
    }
    let a=gradients[index].stops[lower];
    var c=a.exact;
    if lower+1u<settings.x && t>a.offset.x {
        let b=gradients[index].stops[lower+1u];
        let f=(t-a.offset.x)/(b.offset.x-a.offset.x);
        let mixed=mix(a.components,b.components,f);
        if mixed.a<=0. {return vec4f(0);}
        var rgb=mixed.rgb/mixed.a;
        if settings.y==0u {rgb=linear_from_lab(rgb);}
        if settings.y==2u {rgb=decoded_rgb(clamp(rgb,vec3f(0),vec3f(1)));}
        c=vec4f(rgb,mixed.a);
    }
    let xy=vec2u(pixel);
    var rank=0u;
    for(var bit=0u;bit<3u;bit++){
        let x=(xy.x>>bit)&1u;
        let y=(xy.y>>bit)&1u;
        rank=4u*rank+2u*(x^y)+y;
    }
    let noise=((f32(rank)+.5)/64.-.5)/255.;
    let srgb=clamp(encoded_rgb(clamp(c.rgb,vec3f(0),vec3f(1)))+noise,vec3f(0),vec3f(1));
    return vec4f(decoded_rgb(srgb)*c.a,c.a);
}
@fragment fn gradient_fragment(input: VertexOutput)->@location(0) vec4f {
    var shape=shapes[input.record];
    if shape.fill.a<0. {shape.fill=gradient_color(u32(shape.fill.r),input.position.xy);}
    if shape.border.a<0. {shape.border=gradient_color(u32(shape.border.r),input.position.xy);}
    return shade(shape,input);
}
