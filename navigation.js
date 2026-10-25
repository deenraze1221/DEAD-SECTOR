// One-metre navigation grid shared by all enemies. Expanded obstacles keep bodies clear.
export function createNavigation(colliders) {
  const size = 69, offset = 34;
  const blocked = new Uint8Array(size * size);
  const index = (x, z) => (z + offset) * size + x + offset;
  const free = (x, z) => x >= -offset && x <= offset && z >= -offset && z <= offset && !blocked[index(x,z)];
  for (let z = -offset; z <= offset; z++) for (let x = -offset; x <= offset; x++) {
    blocked[index(x,z)] = colliders.some(b => b.minY < 1.8 && b.maxY > 0 && x > b.minX-.4 && x < b.maxX+.4 && z > b.minZ-.4 && z < b.maxZ+.4) ? 1 : 0;
  }
  function nearest(p) {
    const x = Math.max(-offset,Math.min(offset,Math.round(p.x))), z = Math.max(-offset,Math.min(offset,Math.round(p.z)));
    for(let r=0;r<7;r++) for(let dz=-r;dz<=r;dz++) for(let dx=-r;dx<=r;dx++) if(free(x+dx,z+dz)) return [x+dx,z+dz];
    return null;
  }
  return { path(from, to) {
    const a=nearest(from), b=nearest(to); if(!a||!b)return [];
    const start=index(...a), end=index(...b), parents=new Int32Array(size*size).fill(-1), queue=[start]; parents[start]=start;
    for(let head=0;head<queue.length && parents[end]===-1;head++) {
      const current=queue[head], x=current%size-offset, z=Math.floor(current/size)-offset;
      for(const [dx,dz] of [[1,0],[-1,0],[0,1],[0,-1]]) if(free(x+dx,z+dz)) {
        const next=index(x+dx,z+dz); if(parents[next]===-1){parents[next]=current;queue.push(next);}
      }
    }
    if(parents[end]===-1)return [];
    const result=[];for(let n=end;n!==start;n=parents[n])result.push({x:n%size-offset,z:Math.floor(n/size)-offset});
    return result.reverse();
  }};
}
