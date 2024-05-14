// Paste this into the browser to test the websocket connection
const ws = new WebSocket('ws://127.0.0.1/ws');

ws.onopen = event => {
    console.log('sending echo');
    ws.send(JSON.stringify({
        event: 'echo',
        data: {
            message: 'hello'
        }
    }));

    console.log('sending ready');
    ws.send(JSON.stringify({
        event: 'ready',
        data: {
            access_token: 'dev'
        }
    }));
};

ws.onmessage = event => {
    const msg = JSON.parse(event.data);
    console.log('received msg', msg);
}
