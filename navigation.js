// One-metre navigation grid shared by all enemies. Expanded obstacles keep bodies clear.
export function createNavigation(colliders, radius=.4) {
  const size = 199, offset = 99;
  const blocked = new Uint8Array(size * size);
  const index = (x, z) => (z + offset) * size + x + offset;
  const free = (x, z) => x >= -offset && x <= offset && z >= -offset && z <= offset && !blocked[index(x,z)];
  for (let z = -offset; z <= offset; z++) for (let x = -offset; x <= offset; x++) {
    blocked[index(x,z)] = colliders.some(b => b.minY < 1.8 && b.maxY > 0 && x > b.minX-radius && x < b.maxX+radius && z > b.minZ-radius && z < b.maxZ+radius) ? 1 : 0;
  }
  function nearest(p) {
    const x = Math.max(-offset,Math.min(offset,Math.round(p.x))), z = Math.max(-offset,Math.min(offset,Math.round(p.z)));
    for(let r=0;r<7;r++) for(let dz=-r;dz<=r;dz++) for(let dx=-r;dx<=r;dx++) if(free(x+dx,z+dz)) return [x+dx,z+dz];
    return null;
  }
  const fields=new Map();
  return {path(from,to){const a=nearest(from),b=nearest(to);if(!a||!b)return [];const start=index(...a),end=index(...b);let parents=fields.get(end);
  if(!parents){parents=new Int32Array(size*size).fill(-1);parents[end]=end;const queue=[end];for(let head=0;head<queue.length;head++){const current=queue[head],x=current%size-offset,z=Math.floor(current/size)-offset;for(const [dx,dz]of [[1,0],[-1,0],[0,1],[0,-1]])if(free(x+dx,z+dz)){const n=index(x+dx,z+dz);if(parents[n]===-1){parents[n]=current;queue.push(n);}}}fields.set(end,parents);if(fields.size>4)fields.delete(fields.keys().next().value);}
  if(parents[start]===-1)return [];const path=[];for(let n=start;n!==end;){n=parents[n];path.push({x:n%size-offset,z:Math.floor(n/size)-offset});}return path;}};
}
