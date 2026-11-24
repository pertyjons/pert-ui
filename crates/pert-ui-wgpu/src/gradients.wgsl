// shares the analytic coverage function and the shape records at group 0.
struct GradientStop {
    components: vec4f,
    exact: vec4f,
    offset: vec4f,
}
struct GradientRecord {
    coefficients: vec4f,
    axis: vec4f, // physical axis or radii, inner fraction (-1 for linear), padding
    settings: vec4u, // count, space, extend, first stop
}
@group(1) @binding(0) var<storage,read> gradients: array<GradientRecord>;

@group(1) @binding(1) var<storage,read> stops: array<GradientStop>;

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
fn boundary_side(phase:f32, displacement:vec2f, axis:vec4f, coeff:vec2f)->f32 {
    if axis.z<0. {return dot(displacement-phase*axis.xy,coeff);}
    let minor=min(axis.x,axis.y);
    let radius=(axis.z+phase*(1.-axis.z))*minor;
    if radius<0. {return 1.;}
    let uv=displacement*(vec2f(minor)/axis.xy);
    let norm=max(abs(radius),max(abs(uv.x),abs(uv.y)));
    if norm>1e15 || (norm>0. && norm<1e-15) {
        return dot(uv/norm,uv/norm)-(radius/norm)*(radius/norm);
    }
    return dot(uv,uv)-radius*radius;
}
fn gradient_color(index:u32,pixel:vec2f)->vec4f {
    let coeff=gradients[index].coefficients;
    let settings=gradients[index].settings;
    let axis=gradients[index].axis;
    let inner=axis.z;
    let delta=(pixel-coeff.xy)*coeff.zw;
    var t=delta.x+delta.y;
    if inner>=0. {
        t=(length((pixel-coeff.xy)/axis.xy)-inner)/(1.-inner);
    } else if axis.y==0. {
        t=(pixel.x-coeff.x)/axis.x;
    } else if axis.x==0. {
        t=(pixel.y-coeff.y)/axis.y;
    }
    var cycle=0.;
    if settings.z!=0u {
        cycle=floor(t);
        let nearest=round(t);
        if abs(t-nearest)<0.001 {
            cycle=select(nearest-1.,nearest,boundary_side(nearest,pixel-coeff.xy,axis,coeff.zw)>=0.);
        }
    }
    let reverse=settings.z==2u && fract(cycle*.5)!=0.;
    t=clamp(select(t-cycle,1.-(t-cycle),reverse),0.,1.);
    var lower=0u;
    for(var i=1u;i<settings.x;i++){
        let offset=stops[settings.w+i].offset.x;
        if offset!=0. && abs(offset-t)<=0.001 {
            let side=boundary_side(cycle+select(offset,1.-offset,reverse),pixel-coeff.xy,axis,coeff.zw);
            if ((!reverse && side < 0.0) || (reverse && side > 0.0)) {break;}
        } else if offset>t {break;}
        lower=i;
    }
    let a=stops[settings.w+lower];
    var c=a.exact;
    if lower+1u<settings.x && t>a.offset.x {
        let b=stops[settings.w+lower+1u];
        let f=clamp((t-a.offset.x)/(b.offset.x-a.offset.x),0.,1.);
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
