import axios from 'axios'

export const get = (url, params) => {
    url = getUrl(url)
    return new Promise((resolve, reject) => {
        axios.get(url, { params })
            .then(response => {
                resolve(response.data)
            })
            .catch(error => {
                reject(error)
            })
    })
}

export const post = (url, params) => {
    url = getUrl(url)
    return new Promise((resolve, reject) => {
        axios.post(url, params)
            .then(response => {
                resolve(response.data)
            }).catch(error => {
                reject(error)
            })
    })
}

export const postForm = (url, formData) => {
    url = getUrl(url)
    return new Promise((resolve, reject) => {
        axios.post(url,
            formData,
            {
                headers: { 'Content-Type': 'multipart/form-data' },
                timeout: 1000 * 60 * 2
            }
        ).then(response => {
            resolve(response.data)
        }).catch(error => {
            reject(error)
        })
    })
}

export const getUrl = (url) => {
    if (process.cordova) {
        const apiServer = process.apiServer
        return `${apiServer}${url}`
    }
    return url
}
