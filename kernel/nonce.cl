/*
	GPU plot generator for Burst coin.
	Author: Cryo
	Bitcoin: 138gMBhCrNkbaiTCmUhP9HLU9xwn5QKZgD
	Burst: BURST-YA29-QCEW-QXC3-BKXDL

	Based on the code of the official miner and dcct's plotgen.
*/

#define HASH_SIZE			32
#define HASHES_PER_SCOOP	2
#define SCOOP_SIZE			(HASHES_PER_SCOOP * HASH_SIZE)
#define SCOOPS_PER_PLOT		4096
#define PLOT_SIZE			(SCOOPS_PER_PLOT * SCOOP_SIZE)
#define HASH_CAP			4096
#define GEN_SIZE            (PLOT_SIZE + 16)

#define BUFFER_LEN          HASH_CAP

__kernel void nonce_step2(__global unsigned char* p_buffer, unsigned int p_size, unsigned long p_startNonce, unsigned long p_address) {
	size_t id = get_global_id(0);
	if(id >= p_size) {
		return;
	}

    unsigned long nonce_id = p_startNonce + id;

	unsigned int offset = GEN_SIZE * id;
	unsigned char buffer[BUFFER_LEN + 16];

    unsigned char out_buffer[BUFFER_LEN];
    unsigned int out_buffer_elements = 0;

    encodeLongBEGlobal(p_buffer, offset + PLOT_SIZE, p_address);
    encodeLongBEGlobal(p_buffer, offset + PLOT_SIZE + 8, nonce_id);

    encodeLongBE(buffer, BUFFER_LEN, p_address);
    encodeLongBE(buffer, BUFFER_LEN + 8, nonce_id);

    unsigned int hash_input_len = 0;
    shabal_context_t context;

    // first 128 hashes
    for (unsigned int i = 0; i < 128; ++i) {
        shabal_init(&context);
        shabal_update(&context, buffer, BUFFER_LEN - hash_input_len, hash_input_len + 16);
        shabal_digest(&context, buffer, BUFFER_LEN - hash_input_len - HASH_SIZE);
        hash_input_len += HASH_SIZE;
    }
    memcpyToGlobal(buffer, 0, p_buffer, offset + PLOT_SIZE - BUFFER_LEN, BUFFER_LEN);

    unsigned int buffer_idx = 0; // keep track of ring buffer
    unsigned int out_offset = offset + PLOT_SIZE - 129 * HASH_SIZE;
	for(unsigned int i = 128; i < HASHES_PER_SCOOP * SCOOPS_PER_PLOT; ++i) {
        shabal_init(&context);
        shabal_update_ring_buffer(&context, buffer, buffer_idx, BUFFER_LEN);

        buffer_idx = (((buffer_idx - HASH_SIZE) % BUFFER_LEN) + BUFFER_LEN) % BUFFER_LEN;
        shabal_digest(&context, buffer, buffer_idx);

        // TODO: use output buffer
        memcpyToGlobal(buffer, buffer_idx, p_buffer, out_offset, HASH_SIZE);
        out_offset -= HASH_SIZE;
    }
}

__kernel void nonce_step3(__global unsigned char* p_buffer, unsigned int p_size) {
	size_t id = get_global_id(0);
	if(id >= p_size) {
		return;
	}

	unsigned int offset = GEN_SIZE * id;
	unsigned char hash[HASH_SIZE];

	shabal_context_t context;
	shabal_init(&context);
	shabal_update_global(&context, p_buffer, offset, GEN_SIZE);
	shabal_digest(&context, hash, 0);

	barrier(CLK_LOCAL_MEM_FENCE);

	unsigned int len = PLOT_SIZE;
	for(unsigned int i = 0 ; i < len ; ++i) {
		p_buffer[offset + i] ^= hash[i % HASH_SIZE];
	}
}