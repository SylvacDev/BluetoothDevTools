//
// Silicon Labs’s Bluetooth Low Energy Demo Application
// Contact: https://www.silabs.com/support.
//
// This is free software distributed under the terms of the MIT license reproduced below.
//
// Copyright (C) 2020 Silicon Laboratories Inc. www.silabs.com
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this
// software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included
// in all copies or substantial portions of the Software.
//
// THIS CODE AND INFORMATION ARE PROVIDED "AS IS" WITHOUT WARRANTY OF ANY KIND,
// EITHER EXPRESSED OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE IMPLIED WARRANTIES OF
// MERCHANTABILITY AND/OR FITNESS FOR A PARTICULAR PURPOSE.
//

#include <unistd.h>
#include <stdlib.h>
#include <stdio.h>
#include <ctype.h>
#include <string.h>
#include <math.h>

#include "cmd_def.h"
#include "uart.h"

//#define DEBUG

#define CLARG_PORT 1
#define CLARG_ACTION 2

#define UART_TIMEOUT 1000

#define MAX_DEVICES 64
int found_devices_count = 0;
bd_addr found_devices[MAX_DEVICES];

enum actions {
    action_none,
    action_scan,
    action_connect,
    action_info,
};
enum actions action = action_none;

typedef enum {
    state_disconnected,
    state_connecting,
    state_connected,
    state_finding_services,
    state_finding_attributes,
    state_listening_measurements,
    state_finish,
    state_last
} states;
states state = state_disconnected;

const char *state_names[state_last] = {
    "disconnected",
    "connecting",
    "connected",
    "finding_services",
    "finding_attributes",
    "listening_measurements",
    "finish"
};

#define FIRST_HANDLE 0x0001
#define LAST_HANDLE  0xffff

#define THERMOMETER_SERVICE_UUID            0x1809
#define THERMOMETER_MEASUREMENT_UUID        0x2a1c
#define THERMOMETER_MEASUREMENT_CONFIG_UUID 0x2902

uint8 primary_service_uuid[] = {0x00, 0x28};

uint16 thermometer_handle_start = 0,
       thermometer_handle_end = 0,
       thermometer_handle_measurement = 0,
       thermometer_handle_configuration = 0;

bd_addr connect_addr;

void usage(char *exe)
{
    printf("%s <COMx|list> <scan|address>\n", exe);
}

void change_state(states new_state)
{
#ifdef DEBUG
    printf("DEBUG: State changed: %s --> %s\n", state_names[state], state_names[new_state]);
#endif
    state = new_state;
}

/**
 * Compare Bluetooth addresses
 *
 * @param first First address
 * @param second Second address
 * @return Zero if addresses are equal
 */
int cmp_bdaddr(bd_addr first, bd_addr second)
{
    int i;
    for (i = 0; i < sizeof(bd_addr); i++) {
        if (first.addr[i] != second.addr[i]) return 1;
    }
    return 0;
}

void print_bdaddr(bd_addr bdaddr)
{
    printf("%02x:%02x:%02x:%02x:%02x:%02x",
            bdaddr.addr[5],
            bdaddr.addr[4],
            bdaddr.addr[3],
            bdaddr.addr[2],
            bdaddr.addr[1],
            bdaddr.addr[0]);
}

void print_raw_packet(struct ble_header *hdr, unsigned char *data)
{
    printf("Incoming packet: ");
    int i;
    for (i = 0; i < sizeof(*hdr); i++) {
        printf("%02x ", ((unsigned char *)hdr)[i]);
    }
    for (i = 0; i < hdr->lolen; i++) {
        printf("%02x ", data[i]);
    }
    printf("\n");
}

void output(uint8 len1, uint8* data1, uint16 len2, uint8* data2)
{
    if (uart_tx(len1, data1) || uart_tx(len2, data2)) {
        printf("ERROR: Writing to serial port failed\n");
        exit(1);
    }
}

int read_message(int timeout_ms)
{
    unsigned char data[256]; // enough for BLE
    struct ble_header hdr;
    int r;

    r = uart_rx(sizeof(hdr), (unsigned char *)&hdr, UART_TIMEOUT);
    if (!r) {
        return -1; // timeout
    }
    else if (r < 0) {
        printf("ERROR: Reading header failed. Error code:%d\n", r);
        return 1;
    }

    if (hdr.lolen) {
        r = uart_rx(hdr.lolen, data, UART_TIMEOUT);
        if (r <= 0) {
            printf("ERROR: Reading data failed. Error code:%d\n", r);
            return 1;
        }
    }

    const struct ble_msg *msg = ble_get_msg_hdr(hdr);

#ifdef DEBUG
    print_raw_packet(&hdr, data);
#endif

    if (!msg) {
        printf("ERROR: Unknown message received\n");
        exit(1);
    }

    msg->handler(data);

    return 0;
}

void enable_indications(uint8 connection_handle, uint16 client_configuration_handle)
{
    uint8 configuration[] = {0x02, 0x00}; // enable indications
    ble_cmd_attclient_attribute_write(connection_handle, thermometer_handle_configuration, 2, &configuration);
}

void ble_rsp_system_get_info(const struct ble_msg_system_get_info_rsp_t *msg)
{
    printf("Build: %u, protocol_version: %u, hardware: ", msg->build, msg->protocol_version);
    switch (msg->hw) {
    case 0x01: printf("BLE112"); break;
    case 0x02: printf("BLED112"); break;
    default: printf("Unknown");
    }
    printf("\n");

    if (action == action_info) change_state(state_finish);
}

void ble_evt_gap_scan_response(const struct ble_msg_gap_scan_response_evt_t *msg)
{
    if (found_devices_count >= MAX_DEVICES) change_state(state_finish);

    int i;
    char *name = NULL;

    // Check if this device already found
    for (i = 0; i < found_devices_count; i++) {
        if (!cmp_bdaddr(msg->sender, found_devices[i])) return;
    }
    found_devices_count++;
    memcpy(found_devices[i].addr, msg->sender.addr, sizeof(bd_addr));

    // Parse data
    for (i = 0; i < msg->data.len; ) {
        int8 len = msg->data.data[i++];
        if (!len) continue;
        if (i + len > msg->data.len) break; // not enough data
        uint8 type = msg->data.data[i++];
        switch (type) {
        case 0x09:
            name = malloc(len);
            memcpy(name, msg->data.data + i, len - 1);
            name[len - 1] = '\0';
        }

        i += len - 1;
    }

    print_bdaddr(msg->sender);
    printf(" RSSI:%u", msg->rssi);

    printf(" Name:");
    if (name) printf("%s", name);
    else printf("Unknown");
    printf("\n");

    free(name);
}

void ble_evt_connection_status(const struct ble_msg_connection_status_evt_t *msg)
{
    // New connection
    if (msg->flags & connection_connected) {
        change_state(state_connected);
        printf("Connected\n");

        // Handle for Temperature Measurement configuration already known
        if (thermometer_handle_configuration) {
            change_state(state_listening_measurements);
            enable_indications(msg->connection, thermometer_handle_configuration);
        }
        // Find primary services
        else {
            change_state(state_finding_services);
            ble_cmd_attclient_read_by_group_type(msg->connection, FIRST_HANDLE, LAST_HANDLE, 2, primary_service_uuid);
        }
    }
}

void ble_evt_attclient_group_found(const struct ble_msg_attclient_group_found_evt_t *msg)
{
    if (msg->uuid.len == 0) return;
    uint16 uuid = (msg->uuid.data[1] << 8) | msg->uuid.data[0];

    // First thermometer service found
    if (state == state_finding_services && uuid == THERMOMETER_SERVICE_UUID && thermometer_handle_start == 0) {
        thermometer_handle_start = msg->start;
        thermometer_handle_end = msg->end;
    }
}

void ble_evt_attclient_procedure_completed(const struct ble_msg_attclient_procedure_completed_evt_t *msg)
{
    if (state == state_finding_services) {
        // Thermometer service not found
        if (thermometer_handle_start == 0) {
            printf("No Health Thermometer service found\n");
            change_state(state_finish);
        }
        // Find thermometer service attributes
        else {
            change_state(state_finding_attributes);
            ble_cmd_attclient_find_information(msg->connection, thermometer_handle_start, thermometer_handle_end);
        }
    }
    else if (state == state_finding_attributes) {
        // Client characteristic configuration not found
        if (thermometer_handle_configuration == 0) {
            printf("No Client Characteristic Configuration found for Health Thermometer service\n");
            change_state(state_finish);
        }
        // Enable temperature notifications
        else {
            change_state(state_listening_measurements);
            enable_indications(msg->connection, thermometer_handle_configuration);
        }
    }
}

void ble_evt_attclient_find_information_found(const struct ble_msg_attclient_find_information_found_evt_t *msg)
{
    if (msg->uuid.len == 2) {
        uint16 uuid = (msg->uuid.data[1] << 8) | msg->uuid.data[0];

        if (uuid == THERMOMETER_MEASUREMENT_UUID) {
            thermometer_handle_measurement = msg->chrhandle;
        }
        else if (uuid == THERMOMETER_MEASUREMENT_CONFIG_UUID) {
            thermometer_handle_configuration = msg->chrhandle;
        }
    }
}

#define THERMOMETER_FLAGS_FAHRENHEIT 0x1
void ble_evt_attclient_attribute_value(const struct ble_msg_attclient_attribute_value_evt_t *msg)
{
    if (msg->value.len < 5) {
        printf("Not enough fields in Temperature Measurement value");
        change_state(state_finish);
    }

    uint8 flags = msg->value.data[0];
    int8 exponent = msg->value.data[4];
    int mantissa = (msg->value.data[3] << 16) | (msg->value.data[2] << 8) | msg->value.data[1];

    float value = mantissa * pow(10, exponent);
    if (exponent >= 0)
        exponent = 0;
    else
        exponent = abs(exponent);
    printf("Temperature: %.*f ", exponent, value);

    if (flags & THERMOMETER_FLAGS_FAHRENHEIT)
        printf("F");
    else
        printf("C");
    printf("\n");
}

void ble_evt_connection_disconnected(const struct ble_msg_connection_disconnected_evt_t *msg)
{
    change_state(state_disconnected);
    printf("Connection terminated, trying to reconnect\n");
    change_state(state_connecting);
    ble_cmd_gap_connect_direct(&connect_addr, gap_address_type_public, 40, 60, 100,0);
}

int main(int argc, char *argv[]) {
    char *uart_port = "";

    // Not enough command-line arguments
    if (argc <= CLARG_PORT) {
        usage(argv[0]);
        return 1;
    }

    // COM port argument
    if (argc > CLARG_PORT) {
        if (strcmp(argv[CLARG_PORT], "list") == 0) {
            uart_list_devices();
            return 1;
        }
        else {
            uart_port = argv[CLARG_PORT];
        }
    }

    // Action argument
    if (argc > CLARG_ACTION) {
        int i;
        for (i = 0; i < strlen(argv[CLARG_ACTION]); i++) {
            argv[CLARG_ACTION][i] = tolower(argv[CLARG_ACTION][i]);
        }

        if (strcmp(argv[CLARG_ACTION], "scan") == 0) {
            action = action_scan;
        }
        else if (strcmp(argv[CLARG_ACTION], "info") == 0) {
            action = action_info;
        }
        else {
            int i;
            short unsigned int addr[6];
            if (sscanf(argv[CLARG_ACTION],
                    "%02hx:%02hx:%02hx:%02hx:%02hx:%02hx",
                    &addr[5],
                    &addr[4],
                    &addr[3],
                    &addr[2],
                    &addr[1],
                    &addr[0]) == 6) {

                for (i = 0; i < 6; i++) {
                    connect_addr.addr[i] = addr[i];
                }
                action = action_connect;
            }
        }
    }
    if (action == action_none) {
        usage(argv[0]);
        return 1;
    }

    bglib_output = output;

    if (uart_open(uart_port)) {
        printf("ERROR: Unable to open serial port\n");
        return 1;
    }

    // Reset dongle to get it into known state
    ble_cmd_system_reset(0);
    uart_close();
    do {
        usleep(500000); // 0.5s
    } while (uart_open(uart_port));

    // Execute action
    if (action == action_scan) {
        ble_cmd_gap_discover(gap_discover_observation);
    }
    else if (action == action_info) {
        ble_cmd_system_get_info();
    }
    else if (action == action_connect) {
        printf("Trying to connect\n");
        change_state(state_connecting);
        ble_cmd_gap_connect_direct(&connect_addr, gap_address_type_public, 40, 60, 100,0);
    }

    // Message loop
    while (state != state_finish) {
        if (read_message(UART_TIMEOUT) > 0) break;
    }

    uart_close();

    return 0;
}
