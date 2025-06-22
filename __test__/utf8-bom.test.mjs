import test from 'ava'
import fs from 'fs'
import { fileURLToPath } from 'url'
import { dirname, join } from 'path'
import { tmpdir } from 'os'
import csv from '../csv-parser.js'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

test('utf8-bom › UTF-8 with BOM should strip BOM from headers', t => {
  return new Promise((resolve, reject) => {
    const results = []
    const tempFile = join(tmpdir(), 'test-utf8-bom.csv')

    // Create a UTF-8 BOM test file
    const utf8BomContent = Buffer.concat([
      Buffer.from([0xEF, 0xBB, 0xBF]), // UTF-8 BOM
      Buffer.from('name,age\nJohn,30\nJane,25', 'utf8')
    ])

    fs.writeFileSync(tempFile, utf8BomContent)

    fs.createReadStream(tempFile)
      .pipe(csv())
      .on('data', (data) => {
        results.push(data)
      })
      .on('end', () => {
        try {
          t.is(results.length, 2, 'Should parse 2 rows')
          t.deepEqual(results[0], { name: 'John', age: '30' }, 'First row should have clean headers without BOM')
          t.deepEqual(results[1], { name: 'Jane', age: '25' }, 'Second row should have clean headers without BOM')

          // Clean up
          fs.unlinkSync(tempFile)
          resolve()
        } catch (error) {
          // Clean up on error
          try {
            fs.unlinkSync(tempFile)
          } catch (e) {
            // Ignore cleanup errors
          }
          reject(error)
        }
      })
      .on('error', (err) => {
        // Clean up on error
        try {
          fs.unlinkSync(tempFile)
        } catch (e) {
          // Ignore cleanup errors
        }
        reject(err)
      })
  })
})

test('utf8-bom › UTF-8 with BOM and headers event', t => {
  return new Promise((resolve, reject) => {
    const results = []
    let headers = null
    const tempFile = join(tmpdir(), 'test-utf8-bom-headers.csv')

    // Create a UTF-8 BOM test file
    const utf8BomContent = Buffer.concat([
      Buffer.from([0xEF, 0xBB, 0xBF]), // UTF-8 BOM
      Buffer.from('first,second,third\na,b,c\n1,2,3', 'utf8')
    ])

    fs.writeFileSync(tempFile, utf8BomContent)

    fs.createReadStream(tempFile)
      .pipe(csv())
      .on('headers', (headerArray) => {
        headers = headerArray
      })
      .on('data', (data) => {
        results.push(data)
      })
      .on('end', () => {
        try {
          t.deepEqual(headers, ['first', 'second', 'third'], 'Headers should be clean without BOM')
          t.is(results.length, 2, 'Should parse 2 rows')
          t.deepEqual(results[0], { first: 'a', second: 'b', third: 'c' }, 'First row should have clean headers')
          t.deepEqual(results[1], { first: '1', second: '2', third: '3' }, 'Second row should have clean headers')

          // Clean up
          fs.unlinkSync(tempFile)
          resolve()
        } catch (error) {
          // Clean up on error
          try {
            fs.unlinkSync(tempFile)
          } catch (e) {
            // Ignore cleanup errors
          }
          reject(error)
        }
      })
      .on('error', (err) => {
        // Clean up on error
        try {
          fs.unlinkSync(tempFile)
        } catch (e) {
          // Ignore cleanup errors
        }
        reject(err)
      })
  })
})

test('utf8-bom › UTF-8 without BOM should work normally', t => {
  return new Promise((resolve, reject) => {
    const results = []
    const tempFile = join(tmpdir(), 'test-utf8-no-bom.csv')

    // Create a UTF-8 file without BOM
    const utf8Content = Buffer.from('name,age\nJohn,30\nJane,25', 'utf8')

    fs.writeFileSync(tempFile, utf8Content)

    fs.createReadStream(tempFile)
      .pipe(csv())
      .on('data', (data) => {
        results.push(data)
      })
      .on('end', () => {
        try {
          t.is(results.length, 2, 'Should parse 2 rows')
          t.deepEqual(results[0], { name: 'John', age: '30' }, 'First row should be parsed correctly')
          t.deepEqual(results[1], { name: 'Jane', age: '25' }, 'Second row should be parsed correctly')

          // Clean up
          fs.unlinkSync(tempFile)
          resolve()
        } catch (error) {
          // Clean up on error
          try {
            fs.unlinkSync(tempFile)
          } catch (e) {
            // Ignore cleanup errors
          }
          reject(error)
        }
      })
      .on('error', (err) => {
        // Clean up on error
        try {
          fs.unlinkSync(tempFile)
        } catch (e) {
          // Ignore cleanup errors
        }
        reject(err)
      })
  })
})
