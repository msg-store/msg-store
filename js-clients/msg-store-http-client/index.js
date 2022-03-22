import { request } from 'http'
import { stringify, parse } from 'query-string'

const route = url => api => `${url}/${api}`

export default function (url) {
  // const api = `${url}/api`;
  const api = route(`${url}/api`)
  const msgRoute = api('msg')
  const groupRoute = api('group')
  const groupDefaultsRoute = api('group-defaults')
  const statsRoute = api('stats')
  const storeRoute = api('store')
  //   const exportRoute = api('export')

  const sendRequest = async options => {
    let { url, method, params, headers, data } = options
    method = method || 'GET'
    if (params) {
      params = stringify(params)
      if (params) {
        url += `?${params}`
      }
    }
    // console.log('params: ', params)
    return await new Promise((resolve, reject) => {
      const req = request(url, { method, headers }, stream => {
        let body = ''
        stream.on('data', chunk => {
          body += chunk.toString()
        }).once('close', () => {
          if (stream.statusCode === 200) {
            // console.log(req.listenerCount('data'))
            resolve({
              statusCode: stream.statusCode,
              data: body
            })
          } else {
            reject(new Error(JSON.stringify({
              url,
              statusCode: stream.statusCode,
              error: body
            })))
          }
        })
      }).once('error', error => {
        reject(error)
      })
      if (data) {
        req.write(data)
      }
      req.end()
    })
  }

  const addMsg = async (priority, msg) => {
    const res = await sendRequest({ url: msgRoute, method: 'POST', data: `priority=${priority}?${msg}` })
    if (res.data) {
      res.data = JSON.parse(res.data)
    }
    return res
  }

  const getMsg = async options => {
    const res = await sendRequest({ url: msgRoute, params: options })
    if (res.data) {
      const [headerString, msg] = res.data.split('?', 2)
      const { uuid, ...headers } = parse(headerString)
      res.data = { uuid, headers, msg }
    }
    return res
  }

  const getNext = async () => await getMsg({ })

  const deleteMsg = async uuid => await sendRequest({ url: msgRoute, method: 'DELETE', params: { uuid } })

  const getStream = async options => {
    return new Promise((resolve, reject) => {
      const params = stringify(options)
      let url = msgRoute
      if (params) {
        url += `?${params}`
      }
      let headersString = ''
      const req = request(url, { method: 'GET' }, async stream => {
        stream.pause()
        await new Promise(resolve => {
          stream.once('readable', () => {
            while (true) {
              const chunk = stream.read(1)
              if (!chunk) {
                break
              }
              const chunkString = chunk.toString()
              if (chunkString === '?') {
                break
              }
              headersString += chunkString
            }
            resolve()
          }).once('error', error => {
            reject(error)
          })
        })
        const headers = parse(headersString)
        resolve({ statusCode: stream.statusCode, msgStream: stream, ...headers })
      })
      req.end()
    })
  }

  const addStream = async options => {
    const { msgStream, ...headers } = options
    const headersString = stringify(headers)
    return await new Promise((resolve, reject) => {
      let body = ''
      const req = request(msgRoute, { method: 'POST' }, response => {
        response.on('data', chunk => {
          body += chunk.toString()
        }).on('close', () => {
          const statusCode = response.statusCode
          if (response.statusCode === 200) {
            const { uuid } = JSON.parse(body)
            resolve({ statusCode, uuid })
          } else {
            resolve({ statusCode, error: body })
          }
        })
      }).once('error', error => {
        reject(error)
      })
      // console.log('headers: ', headersString)
      req.write(`${headersString}?`)

      msgStream.pipe(req).on('close', () => {
        req.end()
      }).on('error', msg => {
        reject(msg)
        // console.log('stream error: ', msg);
      })
    })
  }

  const getGroup = async (options) => {
    const res = await sendRequest({ url: groupRoute, params: { ...options } })
    if (res.data) {
      res.data = JSON.parse(res.data)
    }
    return res
  }

  const deleteGroup = async priority => await sendRequest({ url: groupRoute, method: 'DELETE', params: { priority } })

  const setGroupDefaults = async (priority, options) => {
    const reqOptions = {
      url: groupDefaultsRoute,
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      data: JSON.stringify({ priority, ...options })
    }
    return await sendRequest(reqOptions)
  }
  const getGroupDefaults = async priority => {
    const res = await sendRequest({ url: groupDefaultsRoute, method: 'GET', params: { priority } })
    if (res.data) {
      res.data = JSON.parse(res.data)
    }
    return res
  }
  const deleteGroupDefaults = async priority => await sendRequest({ url: groupDefaultsRoute, method: 'DELETE', params: { priority } })

  const getStats = async () => {
    const res = await sendRequest({ url: statsRoute, method: 'GET' })
    if (res.data) {
      res.data = JSON.parse(res.data)
    }
    return res
  }
  const updateStats = async options => {
    const reqOptions = {
      url: statsRoute,
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json'
      },
      data: JSON.stringify(options)
    }
    const res = await sendRequest(reqOptions)
    if (res.data) {
      res.data = JSON.parse(res.data)
    }
    return res
  }
  const deleteStats = async () => {
    const res = await sendRequest({ url: statsRoute, method: 'DELETE' })
    if (res.data) {
      res.data = JSON.parse(res.data)
    }
    return res
  }

  const getStore = async () => {
    const res = await sendRequest({ url: storeRoute, method: 'GET' })
    if (res.data) {
      res.data = JSON.parse(res.data)
    }
    return res
  }
  const updateStore = async options => {
    const reqOptions = {
      url: storeRoute,
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json'
      },
      data: JSON.stringify(options)
    }
    return await sendRequest(reqOptions)
  }

  // const exportMsgs = async outputDirectory => {
  //     const res = await sendRequest({ url: exportRoute, method: 'GET', params: { outputDirectory } })
  //     if (res.data) {
  //         res.data = JSON.parse(res.data)
  //     }
  //     return res
  // }

  return {
    addMsg,
    getMsg,
    deleteMsg,
    getNext,
    addStream,
    getStream,
    getGroup,
    deleteGroup,
    getStats,
    updateStats,
    deleteStats,
    getStore,
    updateStore,
    setGroupDefaults,
    getGroupDefaults,
    deleteGroupDefaults
    // exportMsgs
  }
}
