/* 004175df FUN_004175df */

int __fastcall FUN_004175df(int param_1)

{
  int local_8;
  
  FUN_0043ebd0((uint *)(param_1 + 0x50),(uint *)&DAT_0044ad26);
  FUN_0043ebd0((uint *)(param_1 + 4),(uint *)&DAT_0044ad26);
  *(undefined4 *)(param_1 + 0x68) = 0;
  *(undefined4 *)(param_1 + 100) = 0;
  *(undefined4 *)(param_1 + 0xb4) = 0;
  _memset((void *)(param_1 + 0x6c),0,0x20);
  *(undefined4 *)(param_1 + 0xac) = *(undefined4 *)(param_1 + 0x6c);
  *(undefined4 *)(param_1 + 0xb0) = *(undefined4 *)(param_1 + 0x70);
  *(undefined4 *)(param_1 + 0xb8) = 0;
  *(undefined4 *)(param_1 + 0x48) = 0;
  *(undefined4 *)(param_1 + 0xd8) = 0;
  *(undefined4 *)(param_1 + 0xdc) = 1;
  SetRect((LPRECT)(param_1 + 200),0,0,0,0);
  *(undefined4 *)(param_1 + 0x3c) = 0xfffffffd;
  *(undefined4 *)(param_1 + 0x40) = 0xfffffffd;
  *(undefined4 *)(param_1 + 0xf4) = 0xfffffffd;
  *(undefined4 *)(param_1 + 0xf8) = 0xfffffffd;
  *(undefined4 *)(param_1 + 0xbc) = 0;
  *(undefined4 *)(param_1 + 0xe0) = 0xfffffffd;
  for (local_8 = 0; local_8 < 4; local_8 = local_8 + 1) {
    *(undefined4 *)(param_1 + 0x1c + local_8 * 4) = 0xfffffffd;
    *(undefined4 *)(param_1 + 0xe4 + local_8 * 4) = 0xfffffffd;
  }
  return param_1;
}
