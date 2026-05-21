/* 00412219 FUN_00412219 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 FUN_00412219(undefined4 param_1,size_t *param_2,char *param_3)

{
  FILE *_File;
  undefined4 uVar1;
  size_t sVar2;
  LPBYTE lpData;
  uint unaff_retaddr;
  int local_128;
  uint local_11c [65];
  uint local_18;
  size_t local_14;
  size_t local_10;
  uint local_c;
  int local_8;
  
  local_18 = DAT_00451a00 ^ unaff_retaddr;
  local_8 = 0;
  _File = (FILE *)FUN_0043e6f2(param_3,"wb");
  if (_File == (FILE *)0x0) {
    uVar1 = 0x2b0001;
  }
  else {
    FUN_0043ebd0(local_11c,(uint *)&DAT_0044bf48);
    local_10 = _strlen((char *)local_11c);
    local_10 = local_10 + 1;
    sVar2 = _fwrite(local_11c,1,local_10,_File);
    local_8 = local_8 + sVar2;
    sVar2 = _fwrite(&DAT_004519e4,4,1,_File);
    local_8 = local_8 + sVar2;
    sVar2 = _fwrite(&DAT_00451690,4,1,_File);
    local_8 = local_8 + sVar2;
    sVar2 = _fwrite(&DAT_00451694,4,1,_File);
    local_8 = local_8 + sVar2;
    local_10 = 0x2700;
    sVar2 = _fwrite(&local_10,4,1,_File);
    local_8 = local_8 + sVar2;
    sVar2 = _fwrite(param_2,0x2700,1,_File);
    local_8 = local_8 + sVar2;
    local_10 = *param_2;
    for (local_c = 0; local_c < param_2[0x32]; local_c = local_c + 1) {
      sVar2 = _fwrite((void *)param_2[local_c + 0x21],4,local_10,_File);
      local_8 = local_8 + sVar2;
    }
    if (param_2[0x8f] != 0) {
      local_10 = param_2[0x7d];
      sVar2 = _fwrite((void *)param_2[0x7e],0xc,local_10,_File);
      local_8 = local_8 + sVar2;
      for (local_c = 0; local_c < param_2[0x32]; local_c = local_c + 1) {
        sVar2 = _fwrite((void *)param_2[local_c + 0x7f],4,local_10,_File);
        local_8 = local_8 + sVar2;
      }
    }
    local_10 = _strlen((char *)param_2[0x9a]);
    if (local_10 == 0) {
      sVar2 = _fwrite(&local_10,4,1,_File);
      local_8 = local_8 + sVar2;
    }
    else {
      local_10 = local_10 + 1;
      sVar2 = _fwrite(&local_10,4,1,_File);
      local_8 = local_8 + sVar2;
      sVar2 = _fwrite((void *)param_2[0x9a],1,local_10,_File);
      local_8 = local_8 + sVar2;
    }
    local_10 = _strlen((char *)param_2[0x9b]);
    if (local_10 == 0) {
      sVar2 = _fwrite(&local_10,4,1,_File);
      local_8 = local_8 + sVar2;
    }
    else {
      local_10 = local_10 + 1;
      sVar2 = _fwrite(&local_10,4,1,_File);
      local_8 = local_8 + sVar2;
      sVar2 = _fwrite((void *)param_2[0x9b],1,local_10,_File);
      local_8 = local_8 + sVar2;
    }
    if (param_2[0x9c] == 0) {
      local_10 = 0;
      sVar2 = _fwrite(&local_10,4,1,_File);
      local_8 = local_8 + sVar2;
    }
    else {
      local_10 = _strlen((char *)param_2[0x9c]);
      if (local_10 == 0) {
        sVar2 = _fwrite(&local_10,4,1,_File);
        local_8 = local_8 + sVar2;
      }
      else {
        local_10 = local_10 + 1;
        sVar2 = _fwrite(&local_10,4,1,_File);
        local_8 = local_8 + sVar2;
        sVar2 = _fwrite((void *)param_2[0x9c],1,local_10,_File);
        local_8 = local_8 + sVar2;
      }
    }
    local_10 = _strlen((char *)param_2[0x9d]);
    if (local_10 == 0) {
      sVar2 = _fwrite(&local_10,4,1,_File);
      local_8 = local_8 + sVar2;
    }
    else {
      local_10 = local_10 + 1;
      sVar2 = _fwrite(&local_10,4,1,_File);
      local_8 = local_8 + sVar2;
      sVar2 = _fwrite((void *)param_2[0x9d],1,local_10,_File);
      local_8 = local_8 + sVar2;
    }
    local_10 = param_2[0x594];
    if (local_10 != 0) {
      sVar2 = _fwrite((void *)param_2[0xe9],0xfc,local_10,_File);
      local_8 = local_8 + sVar2;
    }
    local_10 = param_2[0x5b2];
    if (local_10 != 0) {
      for (local_c = 0; local_c < param_2[0x5b2]; local_c = local_c + 1) {
        sVar2 = _fwrite(*(void **)(param_2[0x5b4] + local_c * 4),0x50,1,_File);
        local_8 = local_8 + sVar2;
        for (local_128 = 0; local_128 < *(int *)(*(int *)(param_2[0x5b4] + local_c * 4) + 0x28);
            local_128 = local_128 + 1) {
          sVar2 = _fwrite((void *)(local_128 * 0x14 +
                                  *(int *)(*(int *)(param_2[0x5b4] + local_c * 4) + 0x2c)),0x14,1,
                          _File);
          local_8 = local_8 + sVar2;
        }
        for (local_128 = 0; local_128 < *(int *)(*(int *)(param_2[0x5b4] + local_c * 4) + 0x30);
            local_128 = local_128 + 1) {
          sVar2 = _fwrite((void *)(local_128 * 0x14 +
                                  *(int *)(*(int *)(param_2[0x5b4] + local_c * 4) + 0x34)),0x14,1,
                          _File);
          local_8 = local_8 + sVar2;
        }
      }
      if (param_2[0x5a2] == 0) {
        local_14 = 0;
        sVar2 = _fwrite(&local_14,4,1,_File);
        local_8 = local_8 + sVar2;
      }
      else {
        local_14 = GetEnhMetaFileBits((HENHMETAFILE)param_2[0x5ac],0,(LPBYTE)0x0);
        sVar2 = _fwrite(&local_14,4,1,_File);
        local_8 = local_8 + sVar2;
        lpData = _malloc(local_14);
        GetEnhMetaFileBits((HENHMETAFILE)param_2[0x5ac],local_14,lpData);
        sVar2 = _fwrite(lpData,1,local_14,_File);
        local_8 = local_8 + sVar2;
        _free(lpData);
      }
    }
    sVar2 = _fwrite(&DAT_00452ea8,4,1,_File);
    local_8 = local_8 + sVar2;
    sVar2 = _fwrite(&DAT_00452ea0,4,1,_File);
    local_8 = local_8 + sVar2;
    sVar2 = _fwrite(&DAT_00452e9c,4,1,_File);
    local_8 = local_8 + sVar2 + 1;
    _fwrite(&local_8,4,1,_File);
    _fclose(_File);
    param_2[0x96] = 1;
    param_2[0x97] = 0;
    uVar1 = 0;
  }
  return uVar1;
}
