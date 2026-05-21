/* 0041d944 FUN_0041d944 */

void __thiscall FUN_0041d944(void *this,undefined4 param_1)

{
  void *pvVar1;
  int local_10;
  int local_c;
  int local_8;
  
  for (local_8 = 0; local_8 < *(int *)((int)this + 200); local_8 = local_8 + 1) {
    if (*(int *)((int)this + local_8 * 4 + 0x84) != 0) {
      _free(*(void **)((int)this + local_8 * 4 + 0x84));
      *(undefined4 *)((int)this + local_8 * 4 + 0x84) = 0;
    }
    if (*(int *)((int)this + local_8 * 4 + 0x1fc) != 0) {
      _free(*(void **)((int)this + local_8 * 4 + 0x1fc));
      *(undefined4 *)((int)this + local_8 * 4 + 0x1fc) = 0;
    }
  }
  if (*(int *)((int)this + 0x1f8) != 0) {
    _free(*(void **)((int)this + 0x1f8));
    *(undefined4 *)((int)this + 0x1f8) = 0;
  }
  *(undefined4 *)((int)this + 500) = 0;
  if (*(int *)((int)this + 0x3a4) != 0) {
    if (*(void **)((int)this + 0x3a4) != (void *)0x0) {
      FUN_0041338b(*(void **)((int)this + 0x3a4),3);
    }
    *(undefined4 *)((int)this + 0x3a4) = 0;
  }
  if (*(int *)((int)this + 0x16b0) != 0) {
    DeleteEnhMetaFile(*(HENHMETAFILE *)((int)this + 0x16b0));
    *(undefined4 *)((int)this + 0x16b0) = 0;
  }
  if (1 < *(uint *)((int)this + 0x165c)) {
    *(undefined4 *)((int)this + 0x165c) = 1;
    pvVar1 = _realloc(*(void **)((int)this + 0x268),*(int *)((int)this + 0x165c) * 0x7fff);
    *(void **)((int)this + 0x268) = pvVar1;
  }
  _memset(*(void **)((int)this + 0x268),0,0x7fff);
  if (1 < *(uint *)((int)this + 0x1660)) {
    *(undefined4 *)((int)this + 0x1660) = 1;
    pvVar1 = _realloc(*(void **)((int)this + 0x26c),*(int *)((int)this + 0x1660) * 0x7fff);
    *(void **)((int)this + 0x26c) = pvVar1;
  }
  _memset(*(void **)((int)this + 0x26c),0,0x7fff);
  if (1 < *(uint *)((int)this + 0x1664)) {
    *(undefined4 *)((int)this + 0x1664) = 1;
    pvVar1 = _realloc(*(void **)((int)this + 0x274),*(int *)((int)this + 0x1664) * 0x7fff);
    *(void **)((int)this + 0x274) = pvVar1;
  }
  *(undefined4 *)((int)this + 0x1650) = 0;
  _memset(*(void **)((int)this + 0x274),0,0x7fff);
  _memset(this,0,0x1f0);
  _memset((void *)((int)this + 0x1f0),0,0x4c);
  _memset((void *)((int)this + 0x16d8),0,0x14);
  *(undefined4 *)((int)this + 0x16ec) = 0;
  if (*(int *)((int)this + 0x270) != 0) {
    _free(*(void **)((int)this + 0x270));
  }
  *(undefined4 *)((int)this + 0x270) = 0;
  *(undefined4 *)((int)this + 0x23c) = 0;
  *(undefined4 *)((int)this + 0x240) = 0;
  *(undefined4 *)((int)this + 0x260) = 0;
  *(undefined4 *)((int)this + 0x248) = 0;
  *(undefined4 *)((int)this + 0x250) = 0;
  *(undefined4 *)((int)this + 0x2308) = 0;
  *(undefined4 *)((int)this + 0x16b4) = 0;
  *(undefined4 *)((int)this + 0x16b8) = 0;
  *(undefined4 *)((int)this + 0x16bc) = 0;
  *(undefined4 *)((int)this + 600) = 0;
  *(undefined4 *)((int)this + 0x25c) = 0;
  *(undefined4 *)((int)this + 0x24c) = 0;
  *(undefined4 *)((int)this + 0x2668) = 0;
  *(undefined4 *)((int)this + 0x266c) = 0;
  *(undefined4 *)((int)this + 0x16f0) = param_1;
  *(undefined4 *)((int)this + 0x16d4) = 0;
  *(undefined4 *)((int)this + 0x23cc) = 0;
  *(undefined4 *)((int)this + 0x23d0) = 0;
  *(undefined4 *)((int)this + 0x26ec) = 0;
  *(undefined4 *)((int)this + 0x1674) = 0;
  *(undefined4 *)((int)this + 0x1670) = 0;
  *(undefined4 *)((int)this + 0x1678) = 0;
  *(undefined4 *)((int)this + 0x1684) = 0xffffffff;
  *(undefined4 *)((int)this + 0x1680) = 0xffffffff;
  *(undefined4 *)((int)this + 0x167c) = 0xffffffff;
  _memset((void *)((int)this + 0x25ec),0,0x68);
  SendMessageA(*(HWND *)((int)this + 0x16f0),0x111,0x8007,(int)this + 0x17e4);
  FUN_0043ed39((char *)((int)this + 0x21e8),(byte *)"%s\\user.genlib");
  if (*(int *)((int)this + 0x2678) != 0) {
    for (local_c = 0; local_c < *(int *)((int)this + 0x2670); local_c = local_c + 1) {
      _free(*(void **)(*(int *)((int)this + 0x2678) + local_c * 4));
    }
    _free(*(void **)((int)this + 0x2678));
    *(undefined4 *)((int)this + 0x2678) = 0;
  }
  _memset((void *)((int)this + 0x1688),0,0x28);
  if (*(int *)((int)this + 0x1668) != 0) {
    _free(*(void **)((int)this + 0x1668));
  }
  pvVar1 = _malloc(0x100);
  *(void **)((int)this + 0x1668) = pvVar1;
  _memset(*(void **)((int)this + 0x1668),0,0x100);
  if (*(int *)((int)this + 0x16cc) != 0) {
    for (local_10 = 0; local_10 < *(int *)((int)this + 0x16c4); local_10 = local_10 + 1) {
      pvVar1 = *(void **)(*(int *)((int)this + 0x16cc) + local_10 * 4);
      if (pvVar1 != (void *)0x0) {
        FUN_0041d8f2(pvVar1,1);
      }
    }
    _memset(*(void **)((int)this + 0x16cc),0,*(int *)((int)this + 0x16c4) << 2);
    *(undefined4 *)((int)this + 0x16c4) = 0;
  }
  if (*(int *)((int)this + 0x16d0) != 0) {
    for (local_8 = 0; local_8 < *(int *)((int)this + 0x16c8); local_8 = local_8 + 1) {
      pvVar1 = *(void **)(*(int *)((int)this + 0x16d0) + local_8 * 4);
      if (pvVar1 != (void *)0x0) {
        FUN_0041d91b(pvVar1,1);
      }
    }
    _memset(*(void **)((int)this + 0x16d0),0,*(int *)((int)this + 0x16c8) << 2);
    *(undefined4 *)((int)this + 0x16c8) = 0;
  }
  return;
}
