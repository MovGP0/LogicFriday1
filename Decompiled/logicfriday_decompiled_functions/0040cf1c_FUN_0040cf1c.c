/* 0040cf1c FUN_0040cf1c */

void FUN_0040cf1c(void)

{
  bool bVar1;
  bool bVar2;
  int local_7c;
  int local_78;
  int local_74;
  int local_70;
  int local_6c;
  int local_68;
  int local_64;
  int local_60;
  int local_5c;
  int local_58;
  int local_54;
  int local_50;
  int local_30;
  int local_2c;
  undefined4 local_28;
  int local_24;
  int local_20;
  int local_1c;
  int local_18;
  undefined4 local_14;
  LRESULT local_10;
  uint local_c;
  undefined4 local_8;
  
  local_c = 0;
  if (((DAT_00452e7c == 0) && (DAT_00452eb0 == 0)) && (DAT_00452ed8 == 0)) {
    bVar1 = false;
  }
  else {
    bVar1 = true;
  }
  if (((DAT_00452e98 == 0) && (DAT_00452e90 == 0)) && (DAT_00452e94 == 0)) {
    bVar2 = false;
  }
  else {
    bVar2 = true;
  }
  local_18 = 0;
  local_8 = 0;
  local_14 = 0;
  local_30 = 0;
  local_2c = 0;
  local_28 = 0;
  local_24 = 0;
  local_20 = 0;
  local_1c = 0;
  local_10 = FUN_0040f186(&DAT_00452ad0,&local_1c);
  if ((local_10 != 0) && (local_1c != 0)) {
    local_18 = *(int *)(local_1c + 0x250);
    local_8 = *(undefined4 *)(local_1c + 0x1650);
    local_14 = *(undefined4 *)(local_1c + 0x16b4);
    if (DAT_00452ef0 != 0) {
      SendMessageA(DAT_00452a2c,0x111,0x8016,(LPARAM)&local_c);
    }
  }
  FUN_00439b85(&DAT_00453e28,&local_30);
  if ((bVar1) || (bVar2)) {
    local_50 = 0;
  }
  else {
    local_50 = 1;
  }
  FUN_0040d2b9(0x81,local_50);
  FUN_0040d2b9(0xa8,(uint)!bVar1);
  if (((local_10 == 0) || (bVar2)) || (bVar1)) {
    local_54 = 0;
  }
  else {
    local_54 = 1;
  }
  FUN_0040d2b9(0xa9,local_54);
  if (((local_10 != 1) || (bVar2)) || ((DAT_00452ed0 != 0 || (bVar1)))) {
    local_58 = 0;
  }
  else {
    local_58 = 1;
  }
  FUN_0040d2b9(0xb9,local_58);
  if ((bVar1) || (local_24 == 0)) {
    local_5c = 0;
  }
  else {
    local_5c = 1;
  }
  FUN_0040d2b9(0xad,local_5c);
  if (((DAT_00452e98 == 0) || (local_24 == 0)) || (bVar1)) {
    local_60 = 0;
  }
  else {
    local_60 = 1;
  }
  FUN_0040d2b9(0xcb,local_60);
  if (((DAT_00452e98 == 0) || (local_30 == 0)) || (bVar1)) {
    local_64 = 0;
  }
  else {
    local_64 = 1;
  }
  FUN_0040d2b9(0xb3,local_64);
  if (((DAT_00452e98 == 0) || (local_20 == 0)) || (bVar1)) {
    local_68 = 0;
  }
  else {
    local_68 = 1;
  }
  FUN_0040d2b9(0xae,local_68);
  if (((DAT_00452e98 == 0) || (local_2c == 0)) || (bVar1)) {
    local_6c = 0;
  }
  else {
    local_6c = 1;
  }
  FUN_0040d2b9(0xb4,local_6c);
  if (((local_10 != 1) || (bVar1)) || ((local_18 != 0 || (bVar2)))) {
    local_70 = 0;
  }
  else {
    local_70 = 1;
  }
  FUN_0040d2b9(0x8008,local_70);
  if (((local_10 != 1) || (bVar1)) || ((local_18 != 0 || (bVar2)))) {
    local_74 = 0;
  }
  else {
    local_74 = 1;
  }
  FUN_0040d2b9(0xa5,local_74);
  if ((((DAT_00452ef0 == 0) || (bVar2)) || (bVar1)) || ((local_c & 0xffff) == 0)) {
    local_78 = 0;
  }
  else {
    local_78 = 1;
  }
  FUN_0040d2b9(0x153,local_78);
  if (((DAT_00452ef0 == 0) || (bVar2)) || ((bVar1 || (local_c >> 0x10 == 0)))) {
    local_7c = 0;
  }
  else {
    local_7c = 1;
  }
  FUN_0040d2b9(0x154,local_7c);
  return;
}
