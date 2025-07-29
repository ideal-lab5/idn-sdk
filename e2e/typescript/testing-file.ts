import { exec } from 'child_process'



export const run = async () => {
  console.log("Executing shell script to force open HRMP channels")
  exec('bash ./open_hrmp.sh', (err, stdout, stdrr) => {
    console.log(stdout)
    console.log(err)
    console.log(stdrr)
  })
  // give time for channels to open (30 seconds)
  await new Promise(f => setTimeout(f, 30000))
  return 2;
}